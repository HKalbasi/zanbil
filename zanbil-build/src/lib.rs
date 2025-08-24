use std::fmt::Write;

use build_rs::input::out_dir;
use fs_extra::dir::CopyOptions;

fn dep_includes() -> Vec<(String, String)> {
    let mut includes = Vec::new();

    for (dep, val) in std::env::vars() {
        if let Some(dep) = dep.strip_prefix("DEP_") {
            if let Some(dep) = dep.strip_suffix("_ZANBIL_INCLUDE") {
                includes.push((dep.to_string(), val));
            }
        }
    }

    includes
}

pub fn build() {
    let mut cc = cc::Build::new();

    let my_name = build_rs::input::cargo_manifest_links().expect("zanbil expects a link name");

    let mut main_rs_file = String::new();

    for (dep, include) in dep_includes() {
        build_rs::output::warning(&format!("{dep} {include}"));
        writeln!(main_rs_file, "extern crate {};", dep.to_lowercase()).unwrap();
        cc.include(include);
    }

    std::fs::write(out_dir().join("generated_lib.rs"), main_rs_file).unwrap();

    for entry in walkdir::WalkDir::new("src") {
        let entry = entry.unwrap();
        let path = entry.path().to_path_buf();
        if path.extension().and_then(|x| x.to_str()) == Some("c") {
            build_rs::output::rerun_if_changed(&path);
            cc.file(&path);
        }
    }

    let my_include = out_dir().join("include").join(&my_name);
    std::fs::create_dir_all(&my_include).unwrap();
    std::fs::remove_dir_all(&my_include).unwrap();
    std::fs::create_dir_all(&my_include).unwrap();

    fs_extra::dir::copy(
        "src/",
        &my_include,
        &CopyOptions::new().copy_inside(true).content_only(true),
    )
    .unwrap();

    cc.compile("main");
    build_rs::output::rustc_link_lib("main");
    build_rs::output::metadata(
        "ZANBIL_INCLUDE",
        &my_include.parent().unwrap().to_string_lossy(),
    );
}
