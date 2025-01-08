use std::fs;
use std::io::Write;
use std::process::Command;

use crate::backends::{
    BatchBackend, BatchBuildResults, BuildCommandOptions, BuildProfile, BuildResult, CommandSpec,
};
use crate::package::App;
use crate::util::errors::LingoError;
use crate::util::execute_command_to_build_result;

pub struct CmakeC;

fn gen_cmake_files(app: &App, options: &BuildCommandOptions) -> BuildResult {
    let build_dir = app.output_root.join("build");
    fs::create_dir_all(&build_dir)?;

    // location of the cmake file
    let app_build_folder = app.src_gen_dir().join(&app.main_reactor_name);
    let _ = std::fs::create_dir_all(&app_build_folder);
    let cmake_file = app_build_folder.clone().join("CMakeLists.txt");

    // create potential files that come from the target properties
    app.properties
        .write_artifacts(&app_build_folder)
        .expect("cannot write artifacts");

    // read file and append cmake include to generated cmake file
    let mut content = fs::read_to_string(&cmake_file)?;
    let include_statement = "\ninclude(./aggregated_cmake_include.cmake)";
    content += include_statement;

    // overwrite cmake file
    let mut f = fs::OpenOptions::new()
        .write(true)
        .open(&cmake_file)
        .expect("cannot open file");
    f.write_all(content.as_ref()).expect("cannot write file");
    f.flush().expect("cannot flush");

    // cmake args
    let mut cmake = Command::new("cmake");
    cmake.arg(format!(
        "-DCMAKE_BUILD_TYPE={}",
        if options.profile == BuildProfile::Release {
            "RELEASE"
        } else {
            "DEBUG"
        }
    ));
    cmake.arg(format!(
        "-DCMAKE_INSTALL_PREFIX={}",
        app.output_root.display()
    ));

    cmake.arg("-DCMAKE_INSTALL_BINDIR=bin");
    cmake.arg(format!(
        "-DLF_SOURCE_DIRECTORY=\"{}\"",
        app.src_dir_path().unwrap().display()
    ));

    cmake.arg(format!(
        "-DLF_PACKAGE_DIRECTORY=\"{}\"",
        app.root_path.display()
    ));

    cmake.arg(format!(
        "-DLF_SOURCE_GEN_DIRECTORY=\"{}\"",
        app.src_gen_dir()
            .join(app.main_reactor_name.clone())
            .display()
    ));

    cmake.arg(format!(
        "-DLF_FILE_SEPARATOR=\"{}\"",
        if cfg!(target_os = "windows") {
            "\\"
        } else {
            "/"
        }
    ));

    cmake.arg(&app_build_folder);
    cmake.arg(format!("-B {}", app_build_folder.display()));
    cmake.current_dir(&build_dir);

    println!("Running cmake command `{:?}`", cmake);
    execute_command_to_build_result(cmake)
}

fn do_cmake_build(results: &mut BatchBuildResults, options: &BuildCommandOptions) {
    // open lingo.toml of the dependency
    // read the version
    // cry loud when it doesn't match out specified version

    results.keep_going(options.keep_going);
    super::lfc::LFC::do_parallel_lfc_codegen(options, results, false);
    if !options.compile_target_code {
        return;
    }
    results
        // generate all CMake files ahead of time
        .map(|app| gen_cmake_files(app, options))
        // Run cmake to build everything.
        .map(|app| {
            let app_build_folder = app.src_gen_dir().join(&app.main_reactor_name);

            // compile everything
            let mut cmake = Command::new("cmake");
            cmake.current_dir(&app_build_folder);
            cmake.args(["--build", "."]);

            // add one target arg for each app
            let name = app
                .main_reactor
                .file_stem()
                .ok_or(LingoError::InvalidMainReactor)?;
            cmake.arg("--target");
            cmake.arg(name);

            execute_command_to_build_result(cmake)
        })
        .map(|app| {
            let bin_source = app
                .src_gen_dir()
                .join(&app.main_reactor_name)
                .join(&app.main_reactor_name);
            fs::rename(bin_source, app.executable_path())?;
            Ok(())
        });
}

impl BatchBackend for CmakeC {
    fn execute_command(&mut self, command: &CommandSpec, results: &mut BatchBuildResults) {
        match command {
            CommandSpec::Build(options) => do_cmake_build(results, options),
            CommandSpec::Clean => {
                results.par_map(|app| {
                    crate::util::default_build_clean(&app.output_root)?;
                    Ok(())
                });
            }
            _ => todo!(),
        }
    }
}
