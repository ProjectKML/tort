use std::{fs, path::Path, process::Command};

use spirv_builder::{MetadataPrintout, SpirvBuilder};

//HACK(BeastLe9enD/mxrlxn): Compiling the C files by the RPSL compiler on some platforms causes compile errors due to wrong code generation. This fixes the file to be compileable everywhere
fn fix_file(path: &Path) {
    let contents = fs::read_to_string(path).unwrap();
    fs::write(
        path,
        contents
            .replace(
                "#ifdef _MSC_VER  /* Can only support \"linkonce\" vars with GCC */",
                "#if defined(_MSC_VER) || defined(__MINGW32__)"
            )
            .replace("__forceinline", "__TORT_FIX_FORCEINLINE")
            .replace(
                "#ifndef _MSC_VER\n#define __TORT_FIX_FORCEINLINE __attribute__((always_inline)) inline\n#endif",
                "#ifndef _MSC_VER\n#define __TORT_FIX_FORCEINLINE __attribute__((always_inline)) inline\n#else\n#define __TORT_FIX_FORCEINLINE __forceinline\n#endif"
            ).replace("__declspec(dllexport)", "__RPSL_FIX_DLLEXPORT")
            .replace("typedef unsigned char bool;\n#endif",
                     "typedef unsigned char bool;\n#endif\n#ifdef _WIN32\n#define __RPSL_FIX_DLLEXPORT __declspec(dllexport)\n#else\n#define __RPSL_FIX_DLLEXPORT __attribute__((visibility(\"default\")))\n#endif")
    ).unwrap();
}

fn build_rpsl_modules() {
    let graphs_path = Path::new("src/render_graph/graphs");

    let output_dir = "../../target/rpsl";
    fs::create_dir_all(output_dir).unwrap();

    let compiled_files = walkdir::WalkDir::new(graphs_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .map(|f| {
            let graph_path = f.path();
            let parent_path = graph_path.parent().unwrap();
            println!("cargo:rerun-if-changed={}", graph_path.to_str().unwrap());

            let relative_path = pathdiff::diff_paths(parent_path, graphs_path).unwrap();

            let target_path = Path::new(output_dir).join(relative_path);

            #[cfg(not(feature = "skip-compile"))]
            {
                let output = Command::new("rps-hlslc")
                    .arg(graph_path.to_str().unwrap())
                    .arg("-od")
                    .arg(target_path.to_str().unwrap())
                    .arg("-td")
                    .arg(output_dir)
                    .output()
                    .unwrap();

                if !output.status.success() {
                    panic!("{}", std::str::from_utf8(&output.stdout).unwrap());
                }
            }

            let target_file_path = target_path.join(format!(
                "{}.g.c",
                graph_path.file_name().unwrap().to_str().unwrap()
            ));
            fix_file(&target_file_path);
            target_file_path
        })
        .collect::<Vec<_>>();

    cc::Build::new()
        .files(&compiled_files)
        .compile("tort-render-rpsl");
}

fn main() {
    build_rpsl_modules();

    let _result = SpirvBuilder::new("../tort_shaders", "spirv-unknown-spv1.4")
        .print_metadata(MetadataPrintout::DependencyOnly)
        .multimodule(true)
        .build()
        .unwrap();
}
