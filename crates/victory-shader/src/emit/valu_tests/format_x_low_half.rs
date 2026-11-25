use super::{
    ComputeIo, Dispatch, Host, HostShader, HwStage, NativeProgram, ONE_AT_A_TIME, ResourceType,
    Root, RootParam, RootType, S_ENDPGM, StageIo, UsedResource, View, f16_to_f32,
    lower_test_program, s_load_dwordx4, s_mov_b32,
};

/// Three adjacent scalar format loads feed the low-half input of three
/// mixed-precision FMAs. Binding an SRV selects the source element format.
fn format_x_low_half_program(d16: bool) -> HostShader {
    const THREADS: u32 = 1024;
    let mut words = vec![s_mov_b32(5, 0x80)];
    let mut input = s_load_dwordx4(8, 4);
    input[1] |= 64;
    words.extend(input);
    let mut output = s_load_dwordx4(12, 4);
    output[1] |= 96;
    words.extend(output);
    let op = if d16 { 0x80 } else { 0x00 };
    for component in 0..3 {
        let index = if component == 0 {
            0
        } else {
            // v_add_nc_u32 v4, component, v0 (v0 is the local thread ID).
            words.push(0x25 << 25 | 4 << 17 | (128 + component));
            4
        };
        let dest = 1 + component;
        words.extend([
            0x38 << 26 | op << 18 | 1 << 13,
            0x8002_0000 | dest << 8 | index,
        ]);
    }
    // v_fma_mix_f32: add the decoded low half to one coordinate.
    for (dest, addend) in [(1, 244), (2, 243), (3, 240)] {
        words.extend([
            0xcc20_0000 | dest,
            1 << 27 | addend << 18 | 242 << 9 | (256 + dest),
        ]);
    }
    // Store three coordinates from v1..v3 at byte offset 16 * thread ID.
    words.push(0x1a << 25 | 4 << 17 | 132);
    words.extend([0x38 << 26 | 0x1f << 18 | 1 << 12, 0x8003_0104]);
    words.push(S_ENDPGM);
    let view = |kind, dword| UsedResource {
        kind,
        root: RootType::Table,
        table_offset_dwords: dword,
        lower_bound: 0,
        stride: 1,
        dwords: 4,
        user_data: 4,
        slot: 1,
    };
    let native = NativeProgram {
        stage: HwStage::Cs,
        wave64: false,
        vgprs: 5,
        layout: vec![
            RootParam {
                root: RootType::Constants,
                table_offset_dwords: 0,
                sgprs: 4,
                base_register: 0,
                user_data: 0,
                slot: 0,
            },
            RootParam {
                root: RootType::Table,
                table_offset_dwords: 0,
                sgprs: 1,
                base_register: 0,
                user_data: 4,
                slot: 1,
            },
        ],
        used: vec![view(ResourceType::Srv, 16), view(ResourceType::Uav, 24)],
        io: StageIo::Cs(ComputeIo {
            rsrc2: 5 << 1,
            threads: [THREADS, 1, 1],
        }),
        code: words.iter().flat_map(|word| word.to_le_bytes()).collect(),
    };
    lower_test_program(&native)
}

#[test]
fn format_x_low_half_arithmetic_on_hardware() {
    check_format_x_low_half(false);
}

#[test]
fn format_x_low_half_arithmetic_on_warp() {
    check_format_x_low_half(true);
}

fn check_format_x_low_half(warp: bool) {
    use crate::emit::host::Stride;
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_R16_FLOAT, DXGI_FORMAT_R16_UINT};
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory4};

    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let Ok(host) = Host::new(warp) else {
        eprintln!("SM 6.6 adapter unavailable (WARP={warp})");
        return;
    };
    // SAFETY: the LUID comes from the test's live D3D12 device.
    let adapter = unsafe {
        let factory: IDXGIFactory4 = CreateDXGIFactory1().expect("DXGI factory");
        let adapter: IDXGIAdapter1 = factory
            .EnumAdapterByLuid(host.device.GetAdapterLuid())
            .expect("adapter for the test device");
        adapter.GetDesc1().expect("DXGI adapter description")
    };
    let end = adapter.Description.iter()
        .position(|&c| c == 0)
        .unwrap_or(adapter.Description.len());
    eprintln!("Test device: {}", String::from_utf16_lossy(&adapter.Description[..end]));
    let plain = format_x_low_half_program(false);
    let d16 = format_x_low_half_program(true);
    assert!(plain.half_views.is_empty());
    assert_eq!(d16.half_views, [(1, 16)]);
    let finite: Vec<_> = (0..=u16::MAX)
        .filter(|v| v & 0x7c00 != 0x7c00)
        .collect();
    assert_eq!(finite.len(), 63_488);
    let label = if warp { "WARP" } else { "hardware" };
    let mut mismatches = Vec::new();
    for (name, shader, format) in [
        ("ordinary/FLOAT", &plain, DXGI_FORMAT_R16_FLOAT),
        ("ordinary/UINT", &plain, DXGI_FORMAT_R16_UINT),
        ("explicit-d16/UINT", &d16, DXGI_FORMAT_R16_UINT),
    ] {
        let (root, pso) = host.compute_pipeline(shader).expect("compute pipeline");
        let ud = vec![0u32; shader.user_data_dwords().max(1) as usize];
        let mut correct = 0;
        let mut first_mismatch = None;
        for begin in (0..finite.len()).step_by(1024) {
            let input: Vec<_> = (0..1026)
                .map(|i| finite[(begin + i) % finite.len()])
                .collect();
            let data: Vec<_> = input
                .chunks_exact(2)
                .map(|v| u32::from(v[0]) | u32::from(v[1]) << 16)
                .collect();
            let pad = || View::Buffer {
                data: &[0],
                stride: Stride::Format(DXGI_FORMAT_R16_UINT, 2),
            };
            let views = [
                pad(),
                pad(),
                View::Buffer { data: &data, stride: Stride::Format(format, 2) },
                View::RawUav(4096),
            ];
            let out = host
                .dispatch(&root, &pso, &Dispatch {
                    roots: vec![(0, Root::Constants(ud.clone()))],
                    views: &views,
                    samplers: &[],
                    groups: [1, 1, 1],
                })
                .expect("dispatch");
            assert_eq!(out[0].len(), 4096);
            for (lane, coords) in out[0].chunks_exact(4).enumerate() {
                for (axis, addend) in [2.0, -1.0, 0.5].into_iter().enumerate() {
                    let half = input[lane + axis];
                    let low = if format == DXGI_FORMAT_R16_FLOAT {
                        f16_to_f32(half).to_bits() as u16
                    } else {
                        half
                    };
                    let want = (f16_to_f32(low) + addend).to_bits();
                    if coords[axis] == want {
                        correct += 1;
                    } else if first_mismatch.is_none() {
                        first_mismatch = Some((half, axis, coords[axis], want));
                    }
                }
            }
        }
        let total = finite.len() * 3;
        eprintln!("{label} {name}: {correct}/{total} coordinate results matched");
        if let Some(example) = first_mismatch {
            mismatches.push((name, total - correct, example));
        }
    }
    assert!(mismatches.is_empty(), "{label} mismatches: {mismatches:08x?}");
}
