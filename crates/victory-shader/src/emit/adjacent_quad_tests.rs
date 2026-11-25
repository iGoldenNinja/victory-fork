//! Compare DPP quad reads against native quad operations and a coordinate oracle
//! on either side of a diagonal triangle edge. The guest program is hand-authored.
use super::*;
use crate::native::{HwStage, NativeProgram, StageIo};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory4};

const QUAD_PS: &str = r#"
[RootSignature(RS)] float4 ps(V i) : SV_Target {
    float v = i.pos.x + 2.0 * i.pos.y;
    v *= v;
    return float4(QuadReadAcrossX(v), QuadReadAcrossY(v),
                  QuadReadAcrossDiagonal(v), QuadReadLaneAt(v, 0));
}"#;

const WAVE_PS: &str = r#"
[RootSignature(RS)] float4 ps(V i) : SV_Target {
    float v = i.pos.x + 2.0 * i.pos.y;
    v *= v;
    uint lane = WaveGetLaneIndex();
    return float4(WaveReadLaneAt(v, lane ^ 1), WaveReadLaneAt(v, lane ^ 2),
                  WaveReadLaneAt(v, lane ^ 3), WaveReadLaneAt(v, lane & ~3u));
}"#;

// Each vertex has a float4 position and a float2 texture coordinate.
#[rustfmt::skip]
const FULL_TRIANGLE: [f32; 18] = [
    -1., -1., 0.5, 1., 0.,  1.,
    -1.,  3., 0.5, 1., 0., -1.,
     3., -1., 0.5, 1., 2.,  1.,
];
#[rustfmt::skip]
const EDGE_TRIANGLE: [f32; 18] = [
    -1.,  -1.,  0.5, 1., 0.,    1.,
    -1.,   1.1, 0.5, 1., 0.,   -0.05,
     1.1, -1.,  0.5, 1., 1.05,  1.,
];

fn guest_pixel(signature: &[u8]) -> HostShader {
    // Pixel centers X and Y arrive in v0 and v1. v2 = (x + y + y)^2.
    let vop2 = |op: u32, dst: u32, a: u32, b: u32| (op << 25) | (dst << 17) | (b << 9) | (256 + a);
    let mut words = vec![vop2(3, 2, 1, 1), vop2(3, 2, 0, 2), vop2(8, 2, 2, 2)];

    // v_mov_b32 DPP from v2. The four 2-bit selectors name source lanes.
    for (dst, perm) in [(3, 0xb1), (4, 0x4e), (5, 0x1b), (6, 0x00)] {
        words.extend([0x7e00_02fa | (dst << 17), 0xff00_0002 | (perm << 8)]);
    }
    words.extend([0xf800_180f, 0x0605_0403, 0xbf81_0000]); // exp mrt0, v3:v6; s_endpgm

    let native = NativeProgram {
        stage: HwStage::Ps,
        wave64: false,
        vgprs: 7,
        layout: vec![],
        used: vec![],
        io: StageIo::Ps {
            inputs: vec![],
            input_addr: (1 << 8) | (1 << 9),
            db_shader_control: 0,
            patches: vec![],
        },
        code: words.into_iter().flat_map(u32::to_le_bytes).collect(),
    };
    let container = crate::dxbc::Container::parse(signature).unwrap();
    let (program, structured) =
        crate::range::build(&native.patched_code(), native.wave64, native.vgprs).unwrap();
    let structured = structured.unwrap();
    let resources =
        crate::resource::track(&program, &native.layout, native.stage, Some(&structured)).unwrap();
    let io = crate::io::raise(&program, &native).unwrap();
    super::super::emit(&container, &native, &program, &structured, &io, &resources).unwrap()
}

fn expected(x: usize, y: usize) -> [f32; 4] {
    // Input SV_Position refers to pixel centers, at (x + 0.5, y + 0.5).
    let value = |x: usize, y: usize| {
        let v = x as f32 + 2.0 * y as f32 + 1.5;
        v * v
    };
    [
        value(x ^ 1, y),
        value(x, y ^ 1),
        value(x ^ 1, y ^ 1),
        value(x & !1, y & !1),
    ]
}

fn run_adjacent(warp: bool) {
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!(
        "victory-adjacent-quad-{}-{warp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let dxc =
        PathBuf::from(std::env::var_os("VICTORY_PC_DXC").expect("set VICTORY_PC_DXC to dxc.exe"));
    let source = format!("{PS_COMMON}\n{QUAD_PS}");
    let vs = compile(&dxc, &dir, &source, "vs", "vs_6_5").unwrap();
    let reference = compile(&dxc, &dir, &source, "ps", "ps_6_5").unwrap();
    let wave_control = compile(
        &dxc,
        &dir,
        &format!("{PS_COMMON}\n{WAVE_PS}"),
        "ps",
        "ps_6_5",
    )
    .unwrap();
    let mut guest = guest_pixel(&reference);
    validator::Validator::load(validator::Validator::default_path().as_deref())
        .unwrap()
        .validate(&mut guest.dxil)
        .unwrap();

    let host = Host::new(warp).expect("D3D12 shader model 6.6 adapter");
    // SAFETY: the selected device is live and its LUID identifies its DXGI adapter.
    let adapter = unsafe {
        let factory: IDXGIFactory4 = CreateDXGIFactory1().unwrap();
        let adapter: IDXGIAdapter1 = factory
            .EnumAdapterByLuid(host.device.GetAdapterLuid())
            .unwrap();
        adapter.GetDesc1().unwrap()
    };
    let end = adapter
        .Description
        .iter()
        .position(|&ch| ch == 0)
        .unwrap_or(adapter.Description.len());
    eprintln!(
        "adapter {}",
        String::from_utf16_lossy(&adapter.Description[..end])
    );

    // SAFETY: DXC compiled the reference with the root signature used by all three shaders.
    let root: ID3D12RootSignature =
        unsafe { host.device.CreateRootSignature(0, &reference) }.unwrap();
    let draw = |shader: &[u8], vertices: &[f32]| {
        let pipeline = host.pipeline_with(&root, &vs, shader).unwrap();
        let args = Draw {
            roots: vec![(0, Root::Constants(vec![0; 4]))],
            textures: &[],
            samplers: &[],
            vertices,
            stride: 24,
            size: (16, 16),
            mesh_groups: 0,
        };
        host.draw(&root, &pipeline, &args).unwrap()
    };

    for (case, vertices, is_edge) in [
        ("full", &FULL_TRIANGLE, false),
        ("edge", &EDGE_TRIANGLE, true),
    ] {
        let quad_pixels = draw(&reference, vertices);
        let guest_pixels = draw(&guest.dxil, vertices);
        let wave_pixels = draw(&wave_control, vertices);
        let mut covered = 0;
        let mut guest_diffs = 0;
        let mut wave_diffs = 0;
        let mut first_guest = None;
        let mut first_wave = None;

        for index in 0..256 {
            let x = index % 16;
            let y = index / 16;
            let oracle = if !is_edge || x <= y {
                covered += 1;
                expected(x, y)
            } else {
                [0.; 4]
            };
            assert_eq!(
                quad_pixels[index], oracle,
                "DXC quad reference at ({x}, {y})"
            );
            if guest_pixels[index] != oracle {
                guest_diffs += 1;
                first_guest.get_or_insert((x, y, guest_pixels[index], oracle));
            }
            if wave_pixels[index] != oracle {
                wave_diffs += 1;
                first_wave.get_or_insert((x, y, wave_pixels[index], oracle));
            }
        }
        eprintln!(
            "{case} covered={covered}/256 guest_differences={guest_diffs}/256 \
             direct_wave_differences={wave_diffs}/256 first_guest={first_guest:?} \
             first_wave={first_wave:?} top_left={:?}",
            guest_pixels[0]
        );
        assert_eq!(guest_diffs, 0, "{case} translated pixel mismatch");
        if !is_edge {
            assert_eq!(wave_diffs, 0, "fully covered wave control");
        }
    }
}

#[test]
#[ignore = "requires PC DXC and native D3D12"]
fn adjacent_quad_pixels_on_hardware() {
    run_adjacent(false);
}

#[test]
#[ignore = "requires PC DXC and D3D12 WARP"]
fn adjacent_quad_pixels_on_warp() {
    run_adjacent(true);
}
