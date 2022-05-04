use std::env;
use std::fs::File;
use glob;
use std::path::{PathBuf};
use std::io::{BufWriter, Write, BufReader, BufRead};
use std::vec::Vec;
use std::process::{Command};
use curl::easy::Easy;
use flate2::read::GzDecoder;
use tar::Archive;

fn get_env(name: &str) -> String {
    env::var(name).unwrap()
}

fn write_envs(env_vars:&Vec<&str>, filename: &str) {
    let f = File::create(filename).unwrap();
    let mut writer = BufWriter::new(f);
    for &env_var in env_vars {
        let var = get_env(env_var);
        writer.write(format!("{}={}\n", env_var, var).as_bytes()).unwrap();
    }
}

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

#[allow(dead_code)]
fn get_c_files(pattern: &str)->Vec<PathBuf> {
    let iter = glob::glob(pattern).unwrap();
    iter.filter(|x| x.is_ok())
        .map(|x| x.unwrap()).collect()
}

fn get_r_src_url() -> String {
    let f = File::open("R_VERSION").unwrap();
    let mut reader = BufReader::new(f);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    format!("https://cran.r-project.org/src/base/R-4/R-{}.tar.gz", line.trim())
}

static SED_CMD: &str = "sed -i 's/HAVE_VISIBILITY_ATTRIBUTE 0/HAVE_VISIBILITY_ATTRIBUTE 1/' ./configure";

fn main() {
    let env_vars = vec![
        "OUT_DIR",
        "TARGET",
        "CARGO_MAKEFLAGS",
    ];
    write_envs(&env_vars, "build.txt");
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut r_src_path = PathBuf::new();
    r_src_path.push(&out_dir);
    r_src_path.push("R-4.2.0");
    let r_src_path = r_src_path.to_str().unwrap();

    println!("cargo:rerun-if-changed=R_VERSION");
    
    let lib_paths = [
        "src/nmath",
        "src/unix",
        "src/appl",
        "src/extra/tre",
        "bin",
    ];
    for lib_path in lib_paths {
        println!("cargo:rustc-link-search={}/{}", r_src_path, lib_path);
    }

    let libs = [
        "R",
        "nmath",
        "unix",
        "appl",
        "tre",
    ];

    for lib in libs {
        println!("cargo:rustc-link-lib={}", lib);
    }


    // download
    let r_src_url = get_r_src_url();
    println!("url {}", r_src_url);
    let r_src_tgz = download(r_src_url.as_str());
    let r_src_tgz = GzDecoder::new(r_src_tgz.as_slice());
    Archive::new(r_src_tgz).unpack(&out_dir).unwrap();

    // modify visibility
    Command::new("sh")
                .arg("-c")
                .arg(SED_CMD) /* not compile x */
                .current_dir(&r_src_path)
                .output()
                .expect("failed to modify configure script");

    // run configure
    let output = Command::new("sh")
                .arg("-c")
                .arg("./configure --with-x=no") /* not compile x */
                .current_dir(&r_src_path)
                .output()
                .expect("failed to configure R build");

    if !output.status.success() {
        let s = String::from_utf8(output.stderr).expect("Found invalid UTF-8");
        panic!("Configure failed\n{}", s);
    }

    // run configure
    let output = Command::new("sh")
                .arg("-c")
                .arg("./configure --with-x=no") /* not compile x */
                .current_dir(&r_src_path)
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
                .arg(make_cmd)
                .current_dir(&r_src_path)
                .output()
                .expect("failed to build R");
    if !output.status.success() {
        let s = String::from_utf8(output.stderr).expect("Found invalid UTF-8");
        panic!("R build failed\n{}", s);
    }
}
