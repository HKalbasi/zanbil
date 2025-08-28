use std::{fmt::Write, path::PathBuf};

use base64::{Engine, prelude::BASE64_URL_SAFE};
use build_rs::input::out_dir;
use fs_extra::dir::CopyOptions;
use serde::{Deserialize, Serialize};
use toml::Value;

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

#[derive(Debug, Default, Deserialize)]
pub struct ZanbilConf {
    pub cpp: Option<u8>,
    #[serde(default)]
    pub make_dependencies_public: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub include_dirs: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct ZanbilCrate {
    pub name: String,
    pub config: ZanbilConf,
    pub include_dir: PathBuf,
    pub aggregated_include_dirs: Vec<PathBuf>,
    pub dependencies: Vec<Dependency>,
}

pub fn init_zanbil_crate() -> ZanbilCrate {
    let name = build_rs::input::cargo_manifest_links().expect("zanbil expects a link name");

    let cargo_toml_path = build_rs::input::cargo_manifest_dir().join("Cargo.toml");
    build_rs::output::rerun_if_changed(&cargo_toml_path);
    let cargo_toml = std::fs::read_to_string(cargo_toml_path).unwrap();

    let value: Value = toml::from_str(&cargo_toml).unwrap();

    let config: ZanbilConf = value
        .get("package")
        .and_then(|x| x.get("metadata")?.get("zanbil")?.clone().try_into().ok())
        .unwrap_or_default();

    let include_dir = out_dir().join("include");
    let mut dependencies: Vec<Dependency> = vec![];
    std::fs::create_dir_all(&include_dir).unwrap();
    std::fs::remove_dir_all(&include_dir).unwrap();
    std::fs::create_dir_all(&include_dir).unwrap();

    let mut main_rs_file = String::new();

    for (dep, include) in dep_includes() {
        writeln!(main_rs_file, "extern crate {};", dep.to_lowercase()).unwrap();
        dependencies.push(toml::from_slice(&BASE64_URL_SAFE.decode(&include).unwrap()).unwrap());
    }

    let mut aggregated_include_dirs: Vec<PathBuf> = dependencies
        .iter()
        .flat_map(|x| &x.include_dirs)
        .chain([&include_dir])
        .cloned()
        .collect();

    aggregated_include_dirs.sort();
    aggregated_include_dirs.dedup();

    std::fs::write(out_dir().join("generated_lib.rs"), main_rs_file).unwrap();

    let me = Dependency {
        include_dirs: if config.make_dependencies_public {
            aggregated_include_dirs.clone()
        } else {
            vec![include_dir.clone()]
        },
    };

    build_rs::output::metadata(
        "ZANBIL_INCLUDE",
        &BASE64_URL_SAFE.encode(toml::to_string(&me).unwrap()),
    );

    ZanbilCrate {
        name,
        config,
        include_dir,
        dependencies,
        aggregated_include_dirs,
    }
}

pub fn build() {
    let zc = init_zanbil_crate();

    let mut cc = cc::Build::new();

    cc.includes(&zc.aggregated_include_dirs);

    let cpp = zc.config.cpp;

    build_rs::output::rerun_if_env_changed("ZANBIL_CXX");
    build_rs::output::rerun_if_env_changed("ZANBIL_CC");

    if let Some(cpp) = cpp {
        if let Ok(cxx) = std::env::var("ZANBIL_CXX") {
            cc.compiler(cxx);
        } else {
            cc.compiler("zanbil_c++");
        }
        cc.cpp(true);
        cc.std(&format!("c++{cpp}"));
    } else {
        if let Ok(cxx) = std::env::var("ZANBIL_CC") {
            cc.compiler(cxx);
        } else {
            cc.compiler("zanbil_cc");
        }
    }

    let c_extension = if cpp.is_some() {
        Some("cpp")
    } else {
        Some("c")
    };

    for entry in walkdir::WalkDir::new("src") {
        let entry = entry.unwrap();
        let path = entry.path().to_path_buf();
        if path.extension().and_then(|x| x.to_str()) == c_extension {
            build_rs::output::rerun_if_changed(&path);
            cc.file(&path);
        }
    }

    let my_include = zc.include_dir.join(&zc.name);

    fs_extra::dir::copy(
        "src/",
        &my_include,
        &CopyOptions::new().copy_inside(true).content_only(true),
    )
    .unwrap();

    cc.compile("main");
    build_rs::output::rustc_link_lib("main");
}
