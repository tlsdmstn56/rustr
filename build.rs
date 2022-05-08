use std::env;
use std::fs::{File, read_to_string};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::io::{BufWriter, Write};
use std::vec::Vec;
use std::process::{Command};
use curl::easy::Easy;
use flate2::read::GzDecoder;
use tar::Archive;
use bindgen;

const R_VERSION_FILENAME: &str = "R_VERSION";

const LIB_PATHS: [&str; 1] = [
    "lib",
];
const LIBS: [&str; 2] = [
    "R",
    "Rblas"
];

/// Header files used for generating bindings
/// 
/// Note: order matters
const HEADERS: [&str; 8] = [
    "include/Rconfig.h",
    "src/include/config.h",
    "include/Rembedded.h",
    "src/include/Defn.h",
    "src/include/Rinterface.h",
    "src/include/R_ext/RStartup.h",
    "src/main/datetime.h",
    "include/R_ext/GraphicsEngine.h",
];


const INCLUDE_DIRS: [&str; 4] = [
    "src/include",
    "src/include/R_ext",
    "src/main",
    "src/unix",
];

fn get_env(name: &str) -> String {
    env::var(name).unwrap()
}

/// fetch data from url and returns as byte string
fn download(url: &str) -> Vec<u8> {
    let mut data = Vec::new();
    let mut easy = Easy::new();
    easy.url(url).unwrap();
    {
        let mut easy2 = easy.transfer();
        easy2.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        }).unwrap();
        easy2.perform().unwrap();
    }
    data
}

fn get_r_version() -> String {
    read_to_string(R_VERSION_FILENAME)
        .expect(format!("Failed to read {}", R_VERSION_FILENAME).as_str()).trim().to_string()
}

fn get_r_src_url(r_version: &str) -> String {
    format!("https://cran.r-project.org/src/base/R-4/R-{}.tar.gz", r_version)
}

/// read config.log and extract compiler definition for setting up rustc cfg
fn setup_rustc_cfg(log_file: &Path) {
    const START_PATTERN: &str = "## confdefs.h. ##";
    let config_log = {
        let s = read_to_string(log_file).unwrap();
        let (_, config_def_part) = s.rsplit_once(START_PATTERN).unwrap();
        String::from(config_def_part)
    };
    for line in config_log.split('\n') {
        let mut line_iter = line.trim().split_whitespace();
        match line_iter.next() {
            Some(header) => {
                if header != "#define" {
                    continue;
                }
            },
            None => continue,
        }
        let key = {
            match line_iter.next() {
                Some(cfg_key) => {
                    let is_valid_symbol = cfg_key
                        .chars()
                        .map(|c| c.is_alphanumeric() || c == '_')
                        .reduce(|a, b| a && b)
                        .unwrap();
                    if is_valid_symbol {
                        cfg_key
                    } else {
                        continue;
                    }
                    
                },
                None => continue,
            }
        };   
        let mut val = {
            match line_iter.next() {
                Some(cfg_val) => cfg_val,
                None => continue,
            }
        };        
        if let Some(new_cfg_val) = val.strip_prefix('\"') {
            val = new_cfg_val;
        }
        if let Some(new_cfg_val) = val.strip_suffix('\"') {
            val = new_cfg_val;
        }
        println!("cargo:rustc-cfg={}=\"{}\"", key.to_lowercase(), val);
    }
}

fn get_r_src_path(r_version: &str, out_dir: &str) -> String {
    let mut r_src_path_buf = PathBuf::new();
    r_src_path_buf.push(&out_dir);
    r_src_path_buf.push(format!("R-{}", r_version).as_str());
    r_src_path_buf.to_str().unwrap().to_owned()
}

fn download_r_src_if_necessary(r_version: &str, r_src_path: &str, out_dir: &str) {
    let r_src_url = get_r_src_url(r_version);
    println!("url {}", r_src_url);
    let r_src_exist = Path::new(r_src_path).exists();
    if !r_src_exist {
        let r_src_tgz = download(r_src_url.as_str());
        let r_src_tgz = GzDecoder::new(r_src_tgz.as_slice());
        Archive::new(r_src_tgz).unpack(out_dir).unwrap();
    }
}

fn configure_make(r_src_path: &str) {
    // configure
    let output = Command::new("sh")
                .arg("-c")
                .arg("./configure --with-x=no --enable-R-shlib") /* not compile x */
                .current_dir(r_src_path)
                .output()
                .expect("failed to configure R build");

    if !output.status.success() {
        let s = String::from_utf8(output.stderr).expect("Found invalid UTF-8");
        panic!("Configure failed\n{}", s);
    }
    
    // make
    let make_cmd = format!("make -j");
    let output = Command::new("sh")
                .arg("-c")
                .arg(&make_cmd)
                .current_dir(r_src_path)
                .output()
                .expect("failed to build R");
    if !output.status.success() {
        let s = String::from_utf8(output.stderr).expect("Found invalid UTF-8");
        panic!("R build failed\n{}", s);
    }
}

/// generate wrapper.h in out_dir and returns the path of the wrapper file
fn generate_wrapper_header(out_dir: &str, r_src_path: &str) -> String {
    // generate header path
    let mut r_header_pattern = PathBuf::new();
    r_header_pattern.push(r_src_path);
    let headers:Vec<PathBuf> = HEADERS.iter().map(|s| r_header_pattern.join(s)).collect();

    // write wrapper.h
    let wrapper_path = Path::new(out_dir).join("wrapper.h");
    let f = File::create(wrapper_path.as_path()).expect("Failed to generate wrapper.h for bindgen");
    let mut writer = BufWriter::new(f);
    writer.write_all("#include <stddef.h>\n".as_bytes()).expect("Failed to write header");
    for header in headers.iter() {
        let line = format!("#include \"{}\"\n", header.to_str().unwrap());
        writer.write_all(line.as_bytes()).expect("Failed to write header");
    }

    String::from(wrapper_path.as_os_str().to_str().unwrap())
}

fn generate_bindings(out_dir: &str, r_src_path: &str) {
    // generate wrapper header 
    let warpper_path = generate_wrapper_header(out_dir, r_src_path);
    let mut r_src_path_buf = PathBuf::new();
    r_src_path_buf.push(r_src_path);

    // setup clang args
    let mut clang_args: Vec<String> = INCLUDE_DIRS.iter()
        .map(|s| r_src_path_buf.join(s).to_owned())
        .map(|s| format!("-I{}", s.to_str().unwrap()))
        .collect();
    clang_args.push(String::from("-std=c11"));

    // generate bindings.rs
    // TODO: add moe blocklist_file to reduce bindings.rs file size
    let bindings = bindgen::Builder::default()
        .header(warpper_path.as_str())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_args(&clang_args)
        .blocklist_file(".*math.h")
        .blocklist_file(".*tgmath.h")
        .blocklist_file(".*float.h")
        .blocklist_file(".*wchar.h")
        .blocklist_file(".*stdlib.h")
        .blocklist_file(".*bits/floatn.h")
        .blocklist_file(".*bits/fp.*.h")
        .blocklist_file(".*bits/math.*.h")
        .blocklist_file(".*bits/iscanonical.h")
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the out_dir/bindings.rs file.
    let out_path = PathBuf::from(out_dir);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    println!("cargo:rerun-if-changed=R_VERSION");
    
    // set r source path
    let out_dir = get_env("OUT_DIR");
    let r_version = get_r_version();
    let r_src_path = get_r_src_path(r_version.as_str(), out_dir.as_str());

    // download & extract
    download_r_src_if_necessary(r_version.as_str(), r_src_path.as_str(), out_dir.as_str());

    // configure & make
    configure_make(r_src_path.as_str());

    // extract definition to set rust compile-time cfg
    let mut r_config_log_path = PathBuf::from(&r_src_path);
    r_config_log_path.push("config.log");
    setup_rustc_cfg(r_config_log_path.as_path());    


    // set library search path and library
    for lib_path in LIB_PATHS {
        println!("cargo:rustc-link-search={}{}{}", 
            &r_src_path, MAIN_SEPARATOR, &lib_path);
    }
    for lib in LIBS {
        println!("cargo:rustc-link-lib={}", lib);
    }

    // generate bindings
    generate_bindings(out_dir.as_str(), r_src_path.as_str());
}
