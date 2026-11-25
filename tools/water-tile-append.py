"""Build the marker-free RTX 5080 water tile experiment from a local VIctory cache entry."""
import argparse
import ctypes
import hashlib
import os
from pathlib import Path
import re
import struct
import subprocess
import tempfile
import uuid


STOCK_ENTRY = "61dafa05d260cc1068b322b540b967cedd254755bcf9306e47e3d8b95cda7141"
STOCK_DXIL = "06debf8e0528315ad09f2f3cf39325ce6a6d3d9bda48c43512248556369f0dad"
REPAIRED_DXIL = "4d95e57f74ceefdce9930a96abb4d44b1362d52931d7c629b395eea3126b1df1"
REPAIRED_ENTRY = "96af8bbb212e0fb37f03c1a7bff99ae7b44766fb8eccc25403aa2e9d9482fee9"


def verify(data, expected, description):
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError(f"{description} differs from the tested revision")


def per_tile(disassembly):
    # SSA IDs are local to the single signed input pinned above. Retain all resource
    # declarations and the original store's predicate and value. The two counters grow
    # toward opposite ends of the same list. Only the allocation of each slot changes.
    text = "\n".join(line for line in disassembly.splitlines() if line.strip()) + "\n"
    for result in (662, 739):
        pattern = rf"^  %{result} = call i32 @dx\.op\.atomicBinOp\.i32\([^\n]+$"
        text, changes = re.subn(pattern, f"  %{result} = add i32 0, 0", text, flags=re.MULTILINE)
        if changes != 1:
            raise ValueError("expected one work-group reservation for each counter")
    store = re.compile(r"^  call void @dx\.op\.bufferStore\.f32\([^\n]*?\bi32 %784,", re.MULTILINE)
    allocation = [
        "%append_offset = select i1 %615, i32 0, i32 32",
        "%append_rank = call i32 @dx.op.atomicBinOp.i32(i32 78, %dx.types.Handle %90, i32 0, i32 %append_offset, i32 undef, i32 undef, i32 1)",
        "%append_last = sub i32 %744, 1",
        "%append_reverse = sub i32 %append_last, %append_rank",
        "%append_site = select i1 %615, i32 %append_rank, i32 %append_reverse",
        "%append_inbounds = icmp ult i32 %append_rank, %744",
        "%append_bounded = select i1 %append_inbounds, i32 %append_site, i32 -1",
    ]
    def reserve(match):
        return "".join(f"  {line}\n" for line in allocation) + match[0].replace("%784,", "%append_bounded,")
    text, changes = store.subn(reserve, text)
    if changes != 1:
        raise ValueError("expected one packed coordinate store")
    return text.encode()


def assemble(dxc_dir, source):
    """Assemble LLVM text with Microsoft's IDxcAssembler from the chosen x64 DXC release."""
    pointer = ctypes.c_void_p
    hresult = ctypes.c_long
    def check(status):
        if status < 0:
            raise OSError(f"DXC returned HRESULT 0x{status & 0xffffffff:08x}")
    def guid(value):
        return (ctypes.c_ubyte * 16).from_buffer_copy(uuid.UUID(value).bytes_le)
    def method(obj, index, returns, *argtypes):
        vtable = ctypes.cast(obj, ctypes.POINTER(ctypes.POINTER(pointer))).contents
        proc = ctypes.WINFUNCTYPE(returns, pointer, *argtypes)(vtable[index])
        return lambda *args: proc(obj, *args)
    def read_blob(obj):
        addr = method(obj, 3, pointer)()
        size = method(obj, 4, ctypes.c_size_t)()
        return ctypes.string_at(addr, size)
    owned = []
    with os.add_dll_directory(str(dxc_dir)):
        library = ctypes.WinDLL(str(dxc_dir / "dxcompiler.dll"))
        create = library.DxcCreateInstance
        create.argtypes = [pointer, pointer, ctypes.POINTER(pointer)]
        create.restype = hresult
        def instance(clsid, iid):
            obj = pointer()
            check(create(ctypes.byref(guid(clsid)), ctypes.byref(guid(iid)), ctypes.byref(obj)))
            owned.append(obj)
            return obj
        try:
            lib = instance("6245d6af-66e0-48fd-80b4-4d271796748c", "e5204dc7-d18c-4c3c-bdfb-851673980fe7")
            assembler = instance("d728db68-f903-4f80-94cd-dccf76ec7151", "091f7a26-1c1f-4948-904b-e6e3a8a771d5")
            source_bytes = ctypes.create_string_buffer(source)
            blob = pointer()
            check(method(lib, 6, hresult, pointer, ctypes.c_uint32, ctypes.c_uint32, ctypes.POINTER(pointer))(
                source_bytes, len(source), 65001, ctypes.byref(blob)))
            owned.append(blob)
            op = pointer()
            check(method(assembler, 3, hresult, pointer, ctypes.POINTER(pointer))(blob, ctypes.byref(op)))
            owned.append(op)
            status = hresult()
            check(method(op, 3, hresult, ctypes.POINTER(hresult))(ctypes.byref(status)))
            output = pointer()
            check(method(op, 5 if status.value < 0 else 4, hresult, ctypes.POINTER(pointer))(ctypes.byref(output)))
            owned.append(output)
            if status.value < 0:
                raise ValueError(read_blob(output).decode(errors="replace"))
            return read_blob(output)
        finally:
            for obj in reversed(owned):
                method(obj, 2, ctypes.c_ulong)()


def build(cache_entry, destination, dxc_dir):
    data = cache_entry.read_bytes()
    verify(data, STOCK_ENTRY, "stock cache entry")
    size, = struct.unpack_from("<I", data, 8)
    program = data[12:12+size]
    verify(program, STOCK_DXIL, "stock host program")
    if cache_entry.resolve() == destination.resolve():
        raise ValueError("choose a separate output in the test cache")
    if destination.exists():
        verify(destination.read_bytes(), REPAIRED_ENTRY, "existing output")
        return
    with tempfile.TemporaryDirectory() as folder:
        work = Path(folder)
        stock = work / "input.dxil"
        unsigned = work / "unsigned.dxil"
        signed = work / "signed.dxil"
        stock.write_bytes(program)
        text = subprocess.check_output([str(dxc_dir / "dxc.exe"), "-dumpbin", str(stock)]).decode()
        unsigned.write_bytes(assemble(dxc_dir, per_tile(text)))
        subprocess.run([str(dxc_dir / "dxv.exe"), "-o="+str(signed), str(unsigned)], check=True)
        repaired = signed.read_bytes()
    verify(repaired, REPAIRED_DXIL, "signed output")
    output = data[:8] + struct.pack("<I", len(repaired)) + repaired + data[12+size:]
    verify(output, REPAIRED_ENTRY, "rebuilt entry")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as stream:
        tmp = Path(stream.name)
        stream.write(output)
    try:
        try:
            os.link(tmp, destination)
        except FileExistsError:
            verify(destination.read_bytes(), REPAIRED_ENTRY, "concurrent output")
    finally:
        tmp.unlink(missing_ok=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stock", type=Path, help="original VSH1 cache entry")
    parser.add_argument("output", type=Path, help="new entry in a separate private cache")
    parser.add_argument("--dxc-dir", required=True, type=Path, help="x64 directory containing dxc, dxcompiler, dxil and dxv")
    args = parser.parse_args()
    build(args.stock, args.output, args.dxc_dir.resolve())
    print(f"Verified marker-free cache entry {REPAIRED_ENTRY}")
