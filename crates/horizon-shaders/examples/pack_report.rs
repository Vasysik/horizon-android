//! A shader pack through the translator without opening the game: every program of the pack, in
//! the folder of each dimension, loaded the way the game loads it, listed with what translates and
//! what does not, the failures grouped by cause.
//!
//! ```text
//! cargo run --release -p horizon-shaders --example pack_report -- PACK... [options]
//!   PACK        a .zip, a pack folder (or its parent holding `shaders/`), or a folder of packs
//!   --programs  one line per program (default: only the totals and the causes)
//!   --backends  also write every module with the SPIR-V writer wgpu uses on Vulkan (Android)
//!   --stress    also load it with its options at extremes (all switches on or off, values highest or lowest)
//!   --dims 0,-1,1   dimensions to load (default: each distinct folder of the three)
//!   --example N     show up to N full messages per cause (default 1)
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use horizon_shaders::{
    GameInputs, GbuffersInputs, GeometryInputs, Macros, Pack, Post, PostProgram, dimension_folder,
    discover, load_post,
};

struct Options {
    programs: bool,
    backends: bool,
    stress: bool,
    dims: Vec<i32>,
    examples: usize,
}

fn main() {
    let mut paths = Vec::new();
    let mut opts = Options {
        programs: false,
        backends: false,
        stress: false,
        dims: vec![0, -1, 1],
        examples: 1,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--programs" => opts.programs = true,
            "--backends" => opts.backends = true,
            "--stress" => opts.stress = true,
            "--dims" => {
                opts.dims = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|d| d.trim().parse().ok())
                    .collect();
            }
            "--example" => opts.examples = args.next().and_then(|n| n.parse().ok()).unwrap_or(1),
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() {
        eprintln!("usage: pack_report PACK... [--programs] [--backends] [--dims 0,-1,1]");
        std::process::exit(2);
    }
    let mut packs: Vec<(String, PathBuf)> = Vec::new();
    for p in paths {
        if Pack::open(&p).is_ok() {
            let name = p.file_name().map_or_else(
                || p.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            packs.push((name, p));
        } else {
            for f in discover(&p).into_iter().filter(|f| f.problem.is_none()) {
                packs.push((f.name, f.path));
            }
        }
    }
    if packs.is_empty() {
        eprintln!("no packs found");
        std::process::exit(1);
    }
    for (name, path) in packs {
        report(&name, &path, &opts);
    }
}

/// The vertex inputs of the game's geometry, like `gfx/pack_post.rs` writes them.
fn game_inputs() -> GameInputs {
    let terrain = GbuffersInputs {
        declarations: "layout(location = 0) in vec3 hz_v_pos;".into(),
        body: "    hz_Vertex = vec4(hz_v_pos, 1.0);\n    hz_Color = vec4(1.0);\n".into(),
        attributes: vec![("mc_Entity".into(), "vec4(-1.0)".into())],
        // A phone's depth pass, so the report sees what it adds to the pack's varyings.
        after: "    hz_prepass_uv = hz_MultiTexCoord0.xy;\n".into(),
        prepass_uv: true,
        ..Default::default()
    };
    GameInputs {
        terrain: Some(terrain.clone()),
        sky: Some(terrain.clone()),
        celestial: Some(terrain.clone()),
        entities: Some(terrain.clone()),
        clouds: Some(terrain.clone()),
        weather: Some(terrain.clone()),
        hand: Some(terrain.clone()),
        terrain_geometry: Some(GeometryInputs {
            inputs: GbuffersInputs {
                declarations: format!(
                    "layout(set = 3, binding = 1) readonly buffer HzVertices {{ uint hz_vertices[]; }};\n{}",
                    terrain.declarations
                ),
                ..terrain
            },
            pull: "        hz_v_pos = uintBitsToFloat(uvec3(hz_vertices[hz_vertex_index * 6u], \
                   hz_vertices[hz_vertex_index * 6u + 1u], hz_vertices[hz_vertex_index * 6u + 2u]));\n"
                .into(),
            index: "uint(@)".into(),
        }),
    }
}

/// Names of every program `post` loaded, with its group.
fn loaded(post: &Post) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let optional = [
        &post.terrain,
        &post.water,
        &post.sky,
        &post.sky_textured,
        &post.entities,
        &post.block,
        &post.damaged_block,
        &post.entities_translucent,
        &post.particles,
        &post.clouds,
        &post.weather,
        &post.hand,
        &post.shadow,
        &post.shadow_water,
        &post.shadow_entities,
        &post.shadow_block,
    ];
    for p in post
        .programs
        .iter()
        .chain(&post.shadow_comp)
        .chain(optional.into_iter().flatten())
    {
        out.insert(p.name.clone(), format!("{:?}", p.group));
        for (c, _) in &p.compute {
            out.insert(c.clone(), "Compute".into());
        }
    }
    out
}

/// The part of a message that does not depend on the pack: numbers, quoted names and
/// identifiers after a colon are blanked, so the same cause groups across programs and packs.
fn cause(message: &str) -> String {
    let mut out = String::new();
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' | '\'' | '"' => {
                let close = c;
                for d in chars.by_ref() {
                    if d == close {
                        break;
                    }
                }
                out.push_str("<x>");
            }
            d if d.is_ascii_digit() => {
                while chars
                    .peek()
                    .is_some_and(|n| n.is_ascii_alphanumeric() || *n == '.')
                {
                    chars.next();
                }
                out.push('N');
            }
            _ => out.push(c),
        }
    }
    out.chars().take(110).collect()
}

fn stem_kind(name: &str) -> &'static str {
    let base = name.split('_').next().unwrap_or(name);
    let word: String = base
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .to_owned();
    match word.as_str() {
        "gbuffers" => "gbuffers",
        "shadow" => "shadow",
        "shadowcomp" => "shadowcomp",
        "setup" | "begin" | "prepare" | "deferred" | "composite" | "final" => "post",
        _ => "other",
    }
}

fn report(name: &str, path: &Path, opts: &Options) {
    let start = Instant::now();
    let pack = match Pack::open(path) {
        Ok(p) => p,
        Err(e) => {
            println!("== {name}: cannot open: {e}");
            return;
        }
    };
    let mut macros = Macros::standard();
    macros.define_renderer(4);
    let (shaders, feature_notes) = horizon_shaders::read_properties(&pack, &mut macros);
    println!(
        "== {name}: shaders.properties {} keys, {} feature notes, {} ms",
        shaders.entries.len(),
        feature_notes.len(),
        start.elapsed().as_millis()
    );
    let ranked =
        horizon_shaders::Values::profiles_by_cost(&shaders, &horizon_shaders::options(&pack));
    let ranked: Vec<String> = ranked.iter().map(|(n, c)| format!("{n} {c:.1}")).collect();
    println!(
        "   profiles, lightest first (what Auto reads): {}",
        ranked.join(", ")
    );
    let mut folders: Vec<(String, Vec<i32>)> = Vec::new();
    for &d in &opts.dims {
        let f = dimension_folder(&pack, d);
        match folders.iter_mut().find(|x| x.0 == f) {
            Some(x) => x.1.push(d),
            None => folders.push((f, vec![d])),
        }
    }
    let mut causes: BTreeMap<String, (usize, BTreeSet<String>, String)> = BTreeMap::new();
    let (mut total_ok, mut total_fail, mut total_unused) = (0, 0, 0);
    for (folder, dims) in &folders {
        let t = Instant::now();
        let post = load_post(&pack, folder, &shaders, &macros, &game_inputs());
        let loaded = loaded(&post);
        let failed: BTreeMap<&str, &Vec<String>> = post
            .skipped
            .iter()
            .map(|s| (s.name.as_str(), &s.reasons))
            .collect();
        // Every stage file of the folder, by program.
        let mut present: BTreeSet<String> = BTreeSet::new();
        for f in pack.list(folder) {
            for ext in [".vsh", ".fsh", ".gsh", ".csh"] {
                if let Some(stem) = f.strip_suffix(ext) {
                    present.insert(stem.to_owned());
                }
            }
        }
        let (mut ok, mut fail, mut unused) = (0, 0, 0);
        let mut lines = Vec::new();
        let mut kinds: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
        for stem in &present {
            let kind = stem_kind(stem);
            let slot = kinds.entry(kind).or_default();
            if let Some(reasons) = failed.get(stem.as_str()) {
                fail += 1;
                slot[1] += 1;
                let first = reasons.first().cloned().unwrap_or_default();
                let message = first.splitn(3, ": ").last().unwrap_or(&first).to_owned();
                let c =
                    causes
                        .entry(cause(&message))
                        .or_insert((0, BTreeSet::new(), first.clone()));
                c.0 += 1;
                c.1.insert(stem.clone());
                lines.push(format!("  FAIL   {stem}: {first}"));
            } else if loaded.contains_key(stem) {
                ok += 1;
                slot[0] += 1;
                lines.push(format!("  ok     {stem}"));
            } else {
                unused += 1;
                slot[2] += 1;
                lines.push(format!(
                    "  unused {stem} (off, or not a program the game draws)"
                ));
            }
        }
        // Failures of a stem the folder lacks (a compute stage `composite_a`, found by name).
        for (n, reasons) in &failed {
            if !present.contains(*n) {
                fail += 1;
                let first = reasons.first().cloned().unwrap_or_default();
                let message = first.splitn(3, ": ").last().unwrap_or(&first).to_owned();
                let c =
                    causes
                        .entry(cause(&message))
                        .or_insert((0, BTreeSet::new(), first.clone()));
                c.0 += 1;
                c.1.insert((*n).to_owned());
                lines.push(format!("  FAIL   {n}: {first}"));
            }
        }
        total_ok += ok;
        total_fail += fail;
        total_unused += unused;
        println!(
            "-- folder `{folder}` (dimension {dims:?}): {ok} translate, {fail} fail, {unused} unused; \
             {} ms",
            t.elapsed().as_millis()
        );
        let slot = |p: &Option<PostProgram>| p.as_ref().map_or("-".to_owned(), |p| p.name.clone());
        println!(
            "   draws with: terrain {}, water {}, sky {}, entities {}, block {}, hand {}, shadow {}",
            slot(&post.terrain),
            slot(&post.water),
            slot(&post.sky),
            slot(&post.entities),
            slot(&post.block),
            slot(&post.hand),
            slot(&post.shadow),
        );
        let by_kind: Vec<String> = kinds
            .iter()
            .map(|(k, [o, f, u])| format!("{k} {o}/{f}/{u}"))
            .collect();
        println!("   ok/fail/unused by kind: {}", by_kind.join(", "));
        if opts.programs {
            for l in lines {
                println!("{l}");
            }
        }
        if opts.backends {
            backends(&post);
        }
    }
    println!(
        "== {name}: total {total_ok} translate, {total_fail} fail, {total_unused} unused, {} ms",
        start.elapsed().as_millis()
    );
    if opts.stress {
        stress(name, path);
    }
    let mut causes: Vec<_> = causes.into_iter().collect();
    causes.sort_by_key(|c| std::cmp::Reverse(c.1.0));
    for (cause, (n, programs, example)) in causes.iter() {
        let list: Vec<&str> = programs.iter().map(String::as_str).take(6).collect();
        println!("   {n:>3} x {cause}   [{}]", list.join(" "));
        for _ in 0..opts.examples.saturating_sub(0).min(1) {
            println!("         e.g. {example}");
        }
    }
}

/// The pack with its options pushed to extremes (every switch on, every switch off, every valued
/// option at its highest and at its lowest value): the branches other packs' players reach by
/// choosing options, which the defaults do not. Prints what fails in each.
fn stress(name: &str, path: &Path) {
    let Ok(base) = Pack::open(path) else { return };
    let options = horizon_shaders::options(&base);
    for variant in [
        "switches on",
        "switches off",
        "values highest",
        "values lowest",
    ] {
        let Ok(mut pack) = Pack::open(path) else {
            return;
        };
        let mut values = horizon_shaders::Values::default();
        for o in options.values() {
            match variant {
                "switches on" if o.switch && o.tested && !o.name.ends_with("_SHADER") => {
                    values.set(&o.name, "true")
                }
                "switches off" if o.switch && o.tested && !o.name.ends_with("_SHADER") => {
                    values.set(&o.name, "false")
                }
                "values highest" if !o.switch => {
                    if let Some(v) = o.values.last() {
                        values.set(&o.name, v);
                    }
                }
                "values lowest" if !o.switch => {
                    if let Some(v) = o.values.first() {
                        values.set(&o.name, v);
                    }
                }
                _ => {}
            }
        }
        // `GAME_ORDER=1` reads shaders.properties before the values, as the game did; otherwise
        // after, so that `image.*` and `program.*.enabled`, which test the options, see them.
        let mut macros = Macros::standard();
        macros.define_renderer(4);
        let early = std::env::var_os("GAME_ORDER")
            .map(|_| horizon_shaders::read_properties(&pack, &mut macros).0);
        pack.set_values(values);
        let shaders =
            early.unwrap_or_else(|| horizon_shaders::read_properties(&pack, &mut macros).0);
        let folder = dimension_folder(&pack, 0);
        let t = Instant::now();
        let post = load_post(&pack, &folder, &shaders, &macros, &game_inputs());
        let n = loaded(&post).len();
        println!(
            "   stress `{variant}`: {n} translate, {} fail, {} ms",
            post.skipped.len(),
            t.elapsed().as_millis()
        );
        for s in &post.skipped {
            println!(
                "      FAIL {}: {}",
                s.name,
                s.reasons.first().map_or("", String::as_str)
            );
        }
    }
    let _ = name;
}

/// Every module through the SPIR-V writer (Vulkan, what Android runs), counting the failures.
fn backends(post: &Post) {
    let mut modules: Vec<(String, &naga::Module)> = Vec::new();
    let mut all: Vec<&PostProgram> = post.programs.iter().chain(&post.shadow_comp).collect();
    all.extend(
        [
            &post.terrain,
            &post.water,
            &post.sky,
            &post.sky_textured,
            &post.entities,
            &post.block,
            &post.entities_translucent,
            &post.particles,
            &post.clouds,
            &post.weather,
            &post.hand,
            &post.shadow,
            &post.shadow_water,
            &post.shadow_entities,
            &post.shadow_block,
        ]
        .into_iter()
        .flatten(),
    );
    for p in all {
        if let Some(program) = &p.program {
            modules.push((format!("{} vertex", p.name), &program.vertex));
            modules.push((format!("{} fragment", p.name), &program.fragment));
        }
        for (name, c) in &p.compute {
            modules.push((name.clone(), &c.module));
        }
    }
    let (mut bad, mut panics) = (0, 0);
    for (name, module) in &modules {
        let info = match naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(module)
        {
            Ok(i) => i,
            Err(e) => {
                bad += 1;
                println!("   validate error: {name}: {e}");
                continue;
            }
        };
        let Some(entry) = module.entry_points.first() else {
            continue;
        };
        let (stage, point) = (entry.stage, entry.name.clone());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            use naga::back::spv;
            let options = spv::Options {
                lang_version: (1, 3),
                flags: spv::WriterFlags::LABEL_VARYINGS | spv::WriterFlags::FORCE_POINT_SIZE,
                bounds_check_policies: naga::proc::BoundsCheckPolicies {
                    index: naga::proc::BoundsCheckPolicy::Restrict,
                    buffer: naga::proc::BoundsCheckPolicy::Restrict,
                    image_load: naga::proc::BoundsCheckPolicy::Restrict,
                    binding_array: naga::proc::BoundsCheckPolicy::Unchecked,
                },
                force_loop_bounding: true,
                ..Default::default()
            };
            let pipeline = spv::PipelineOptions {
                shader_stage: stage,
                entry_point: point,
            };
            spv::write_vec(module, &info, &options, Some(&pipeline)).map(drop)
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                bad += 1;
                println!("   spv error: {name}: {e}");
            }
            Err(_) => {
                panics += 1;
                println!("   spv PANIC: {name}");
            }
        }
    }
    println!(
        "   spir-v: {} modules, {bad} errors, {panics} panics",
        modules.len()
    );
}
