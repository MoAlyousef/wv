use std::env;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let exe_pth = out_dir.clone();

    println!("cargo:rerun-if-changed=webview/webview.h");
    println!("cargo:rerun-if-changed=webview/webview.cc");

    let mut build = cc::Build::new();
    if !target.contains("windows-gnu") {
        build
            .cpp(true)
            .define("WEBVIEW_STATIC", "")
            .file("webview/webview.cc")
            .flag_if_supported("-w");
    }

    if target.contains("windows") {
        let edge_weview_native =
            "webview/build/native".to_string();
        if target.contains("msvc") {
            let mut include = edge_weview_native.clone();
            include.push_str("/include");
            build.flag("/DWEBVIEW_EDGE");
            build.flag("/std:c++17");
            build.include(include);
        }

        for &lib in &[
            "user32",
            "oleaut32",
            "ole32",
            "version",
            "shell32",
			"advapi32",
			"shlwapi"
        ] {
            println!("cargo:rustc-link-lib={}", lib);
        }

        let wv_arch = if target.contains("x86_64") {
            "x64"
        } else if target.contains("i686") {
            "x86"
        } else {
            "arm64"
        };

        let mut wv_path = manifest_dir;
        if target.contains("msvc") {
            wv_path.push(edge_weview_native);
        } else {
            wv_path.push("webview");
            wv_path.push("build");
            wv_path.push("native");
        }
        wv_path.push(wv_arch);
        let webview2_dir = wv_path.as_path().to_str().unwrap();
        println!("cargo:rustc-link-search={}", webview2_dir);
        println!("cargo:rustc-link-search={}", out_dir.join("../../..").display());
        if target.contains("msvc") {
            println!("cargo:rustc-link-lib=WebView2LoaderStatic");
        } else {
            if !target.contains("aarch64") {
                println!("cargo:rustc-link-lib=WebView2Loader");
                println!("cargo:rustc-link-lib=webview");
                for entry in std::fs::read_dir(wv_path).expect("Can't read DLL dir") {
                    let entry_path = entry.expect("Invalid fs entry").path();
                    let file_name_result = entry_path.file_name();
                    let mut exe_pth = exe_pth.clone();
                    if let Some(file_name) = file_name_result {
                        let file_name = file_name.to_str().unwrap();
                        if file_name.ends_with(".dll") {
                            exe_pth.push("../../..");
                            let mut for_examples_exe_pth = exe_pth.clone();
                            for_examples_exe_pth.push("examples");
                            exe_pth.push(file_name);
                            std::fs::copy(&entry_path, exe_pth.as_path())
                                .expect("Can't copy from DLL dir /target/..");

                            // Copy .dll to examples folder too, in order to run examples when cross compiling from linux.
                            for_examples_exe_pth.push(file_name);
                            std::fs::copy(&entry_path, for_examples_exe_pth.as_path())
                                .expect("Can't copy from DLL dir to /target/../examples");
                        }
                    }
                }
            } else {
                panic!("{:?} not supported yet", target)
            }
        }
    } else if target.contains("apple") {
        build.flag("-DWEBVIEW_COCOA");
        build.flag("-std=c++11");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=framework=WebKit");
    } else if target.contains("linux") || target.contains("bsd") {
        build.flag("-DWEBVIEW_GTK");
        build.flag("-std=c++11");
        let lib = pkg_config::Config::new()
            .atleast_version("2.8")
            .probe("webkit2gtk-4.1")
            .unwrap();
        for path in lib.include_paths {
            build.include(path);
        }
    } else {
        panic!("Unsupported platform");
    }

    if !target.contains("windows-gnu") {
        build.compile("webview");
    }
}
