use std::fs;

use std::process::Command;

use crate::util::command_line::run_and_capture;
use crate::App;

use crate::backends::{
    BatchBackend, BatchLingoCommand, BuildCommandOptions, BuildProfile, BuildResult, CommandSpec,
};
use crate::backends::CommandSpec::Build;

pub struct Cmake;

fn build_single_app(app: &App, options: &BuildCommandOptions) -> BuildResult {
    let build_dir = app.output_root.join("build");
    fs::create_dir_all(&build_dir)?;

    // cmake generation
    {
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
        cmake.arg(format!("-DCMAKE_INSTALL_BINDIR=bin"));
        cmake.arg(format!("-DREACTOR_CPP_VALIDATE=ON"));
        cmake.arg(format!("-DREACTOR_CPP_TRACE=OFF"));
        cmake.arg(format!("-DREACTOR_CPP_LOG_LEVEL=3"));
        cmake.arg(format!("-DLF_SRC_PKG_PATH={}", app.root_path.display()));
        cmake.arg(app.src_gen_dir());
        cmake.arg(format!("-B {}", build_dir.display()));
        cmake.current_dir(&build_dir);
        run_and_capture(&mut cmake)?;
    }

    let cmake_binary_name = app.main_reactor.file_stem().unwrap();

    {
        // compiling
        let mut cmake = Command::new("cmake");
        cmake.current_dir(&build_dir);
        cmake.args(["--build", "."]);
        cmake.arg("--target");
        cmake.arg(cmake_binary_name);
        run_and_capture(&mut cmake)?;
    }

    {
        // installing
        let mut cmake = Command::new("cmake");
        cmake.current_dir(&build_dir);
        cmake.args(["--install", "."]);
        run_and_capture(&mut cmake)?;
    }

    {
        // cleanup: rename executable to match the app name
        let bin_dir = app.output_root.join("bin");
        fs::rename(bin_dir.join(cmake_binary_name),
                   bin_dir.join(&app.name),
        )?;
    }

    Ok(())
}

impl BatchBackend for Cmake {
    fn execute_command(&mut self, command: BatchLingoCommand) -> BuildResult {
        match command.task {
            CommandSpec::Build(mut options) => {
                let do_compile = options.compile_target_code;
                options.compile_target_code = false;
                super::lfc::LFC::do_parallel_lfc_codegen(
                    &options,
                    &command.apps,
                )?;
                if !do_compile {
                    return Ok(())
                }
                options.compile_target_code = true;

                command
                    .apps
                    .iter()
                    .map(|&app| build_single_app(app, &options))
                    .reduce(crate::util::errors::merge)
                    .unwrap_or(Ok(()))
            }
            CommandSpec::Clean => {
                for &app in &command.apps {
                    crate::util::default_build_clean(&app.output_root)?
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
