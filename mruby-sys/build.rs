#![deny(clippy::all, clippy::pedantic)]
#![deny(warnings, intra_doc_link_resolution_failure)]
#![doc(deny(warnings))]

use fs_extra::dir::{self, CopyOptions};
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// vendored mruby version
const MRUBY_REVISION: &str = "bc7c5d3";

/// Path helpers
struct Build;

impl Build {
    fn root() -> String {
        env::var("CARGO_MANIFEST_DIR").unwrap()
    }

    fn build_config() -> String {
        let target = env::var("TARGET").unwrap();
        if target.starts_with("wasm32-") {
            format!("{}/wasm_build_config.rb", &Build::root(),)
        } else {
            format!(
                "{}/{}_build_config.rb",
                &Build::root(),
                &env::var("PROFILE").expect("PROFILE")
            )
        }
    }

    fn ext_source_dir() -> String {
        format!("{}/mruby-sys", &Build::root())
    }

    fn ext_include_dir() -> String {
        format!("{}/include", Build::ext_source_dir())
    }

    fn ext_source_file() -> String {
        format!("{}/src/mruby-sys/ext.c", &Build::ext_source_dir())
    }

    fn mruby_vendored_dir() -> String {
        format!("{}/vendor/mruby-{}", &Build::root(), MRUBY_REVISION)
    }

    fn mruby_source_dir() -> String {
        format!("{}/mruby-{}", &env::var("OUT_DIR").unwrap(), MRUBY_REVISION)
    }

    fn mruby_minirake() -> String {
        format!("{}/minirake", Build::mruby_source_dir())
    }

    fn mruby_include_dir() -> String {
        format!("{}/include", Build::mruby_source_dir())
    }

    fn mruby_build_dir() -> String {
        format!("{}/{}", &env::var("OUT_DIR").unwrap(), "mruby-build")
    }

    fn mruby_out_dir() -> String {
        let target = env::var("TARGET").unwrap();
        if target == "wasm32-unknown-unknown" {
            format!("{}/sys-wasm/lib", &Build::mruby_build_dir())
        } else if target == "wasm32-unknown-emscripten" {
            format!("{}/sys-emscripten/lib", &Build::mruby_build_dir())
        } else {
            format!("{}/sys/lib", &Build::mruby_build_dir())
        }
    }

    fn bindgen_source_header() -> String {
        format!("{}/mruby-sys.h", &Build::ext_include_dir())
    }

    fn patch(patch: &str) -> String {
        format!("{}/vendor/{}", Build::root(), patch)
    }
}

fn main() {
    let opts = CopyOptions::new();
    let _ = dir::remove(Build::mruby_source_dir());
    dir::copy(
        Build::mruby_vendored_dir(),
        env::var("OUT_DIR").unwrap(),
        &opts,
    )
    .unwrap();
    for patch in vec!["0001-Support-parsing-a-Regexp-literal-with-CRuby-options.patch"] {
        println!("cargo:rerun-if-changed={}", Build::patch(patch));
        if !Command::new("bash")
            .arg("-c")
            .arg(format!("patch -p1 < '{}'", Build::patch(patch)))
            .current_dir(Build::mruby_source_dir())
            .status()
            .unwrap()
            .success()
        {
            panic!("Failed to patch mruby sources with {}", patch);
        }
    }

    // Build the mruby static library with its built in minirake build system.
    // minirake dynamically generates some c source files so we can't build
    // directly with the `cc` crate.
    env::set_var("MRUBY_REVISION", MRUBY_REVISION);
    println!("cargo:rustc-env=MRUBY_REVISION={}", MRUBY_REVISION);
    println!("cargo:rerun-if-env-changed=MRUBY_REVISION");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-changed={}", Build::build_config());
    println!("cargo:rerun-if-changed={}/sys.gembox", Build::root());
    if !Command::new(Build::mruby_minirake())
        .arg("--jobs")
        .arg("4")
        .env("MRUBY_BUILD_DIR", Build::mruby_build_dir())
        .env("MRUBY_CONFIG", Build::build_config())
        .current_dir(Build::mruby_source_dir())
        .status()
        .unwrap()
        .success()
    {
        panic!("Failed to build libmruby.a");
    }

    // Set static lib and search path flags so rustc will link libmruby.a
    // into our binary.
    println!("cargo:rustc-link-lib=static=mruby");
    println!("cargo:rustc-link-search=native={}", Build::mruby_out_dir());

    // Build the extension library
    println!("cargo:rerun-if-changed={}", Build::ext_source_file());
    println!(
        "cargo:rerun-if-changed={}/mruby-sys/ext.h",
        Build::ext_include_dir()
    );
    if env::var("TARGET").unwrap().starts_with("wasm32-") {
        cc::Build::new()
            .file(Build::ext_source_file())
            .include(Build::mruby_include_dir())
            .include(Build::ext_include_dir())
            .include(format!(
                "{}/../target/emsdk/fastcomp/emscripten/system/include/libc",
                Build::root()
            ))
            .compile("libmrubysys.a");
    } else {
        cc::Build::new()
            .file(Build::ext_source_file())
            .include(Build::mruby_include_dir())
            .include(Build::ext_include_dir())
            .compile("libmrubysys.a");
    }

    println!("cargo:rerun-if-changed={}", Build::bindgen_source_header());
    let bindings_path: PathBuf = [&env::var("OUT_DIR").unwrap(), "ffi.rs"].iter().collect();
    let mut bindings = bindgen::Builder::default()
        .header(Build::bindgen_source_header())
        .clang_arg(format!("-I{}", Build::mruby_include_dir()))
        .clang_arg(format!("-I{}", Build::ext_include_dir()))
        .clang_arg("-DMRB_DISABLE_STDIO")
        .clang_arg("-DMRB_UTF8_STRING");
    if env::var("TARGET").unwrap().starts_with("wasm32-") {
        bindings = bindings
            .clang_arg("-DMRB_INT32")
            .clang_arg(format!(
                "-I{}/../target/emsdk/fastcomp/emscripten/system/include/libc",
                Build::root()
            ))
            .clang_arg("-fvisibility=default");
    }
    bindings
        .whitelist_function("^mrb.*")
        .whitelist_type("^mrb.*")
        .whitelist_var("^mrb.*")
        .whitelist_var("^MRB.*")
        .whitelist_var("^MRUBY.*")
        .whitelist_var("REGEXP_CLASS")
        .rustified_enum("mrb_vtype")
        .rustified_enum("mrb_lex_state_enum")
        .rustified_enum("mrb_range_beg_len")
        .rustfmt_bindings(true)
        // work around warnings caused by cargo doc interpreting Ruby doc blocks
        // as Rust code.
        // See: https://github.com/rust-lang/rust-bindgen/issues/426
        .generate_comments(false)
        .generate()
        .expect("Unable to generate mruby bindings")
        .write_to_file(bindings_path)
        .expect("Unable to write mruby bindings");
}
