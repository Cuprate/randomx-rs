// Copyright 2019. The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::{env, path::PathBuf, process::Command};

use cmake::Config;

#[allow(clippy::too_many_lines)]
fn main() {
    let randomx_path = Config::new(env::var("RANDOMX_DIR").unwrap_or_else(|_| "RandomX".to_string()))
        .define("DARCH", "native")
        .build();

    println!("cargo:rustc-link-search=native={}/lib64", randomx_path.display());
    println!("cargo:rustc-link-search=native={}/lib", randomx_path.display());
    println!("cargo:rustc-link-lib=static=randomx");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or("linux".to_string());
    let crt_kind = if links_static_crt() { "static" } else { "dylib" };

    let (link_kind, lib_name) = match target_os.as_str() {
        "macos" | "ios" => ("dylib", "c++"), // Apple targets reject static linking
        "windows" => ("dylib", "msvcrt"),    // Use MSVC runtime on Windows
        "freebsd" | "openbsd" => (crt_kind, "c++"),      // FreeBSD and OpenBSD use "c++"
        _ => (crt_kind, "stdc++"),           // Default for other systems (Linux, etc.)
    };

    if link_kind == "static" {
        if let Some(dir) = cpp_lib_dir(lib_name) {
            println!("cargo:rustc-link-search=native={dir}");
        }
    }
    println!("cargo:rustc-link-lib={link_kind}={lib_name}");

    if target_os == "windows" {
        println!("cargo:rustc-link-lib=advapi32");
    }
}

/// `true` when the target links a static C runtime.
fn links_static_crt() -> bool {
    let mut cmd = Command::new(env::var("RUSTC").unwrap_or_else(|_| "rustc".into()));
    cmd.args(["--print", "cfg", "--target"])
        .arg(env::var("TARGET").unwrap_or_default());
    if let Ok(flags) = env::var("CARGO_ENCODED_RUSTFLAGS") {
        cmd.args(flags.split('\x1f').filter(|arg| !arg.is_empty()));
    }
    match cmd.output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|line| line == r#"target_feature="crt-static""#),
        Err(_) => false,
    }
}

/// Ask the C++ compiler where its runtime is.
fn cpp_lib_dir(lib: &str) -> Option<String> {
    let compiler = cc::Build::new().cpp(true).try_get_compiler().ok()?;
    let out = Command::new(compiler.path())
        .args(compiler.args())
        .arg(format!("-print-file-name=lib{lib}.a"))
        .output()
        .ok()?;
    // A bare filename comes back when the library cannot be found.
    let path = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    if !path.is_absolute() {
        return None;
    }
    Some(path.parent()?.to_str()?.to_owned())
}
