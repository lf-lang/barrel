use crate::util::command_line::run_and_capture;
use crate::App;

use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::write;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

///
/// taken from: https://www.lf-lang.org/docs/handbook/target-declaration?target=c
///
#[derive(Serialize, Deserialize, Clone)]
pub struct Properties {}

///
/// struct which is given to lfc for code generation
///
#[derive(Serialize, Deserialize, Clone)]
pub struct LFCProperties {
    pub src: PathBuf,
    pub out: PathBuf,
    pub properties: HashMap<String, serde_json::Value>,
}

///
/// this struct contains everything that is required to invoke lfc
///
#[derive(Clone)]
pub struct CodeGenerator {
    pub lfc: PathBuf,
    pub properties: LFCProperties,
}

impl Display for LFCProperties {
    /// convert lfc properties to string
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let string = serde_json::to_string(&self).map_err(|_| std::fmt::Error)?;
        write!(f, "{}", string)
    }
}

impl LFCProperties {
    /// root points to root of project
    pub fn new(
        src: PathBuf,
        out: PathBuf,
        mut properties: HashMap<String, serde_json::Value>,
    ) -> LFCProperties {
        properties.insert("no-compile".to_string(), serde_json::Value::Bool(true));
        LFCProperties {
            src,
            out,
            properties,
        }
    }

    /// write lfc properties to file
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        write(path, self.to_string())
    }
}

impl CodeGenerator {
    pub fn new(
        src: PathBuf,
        out: PathBuf,
        lfc: Option<PathBuf>,
        properties: HashMap<String, serde_json::Value>,
    ) -> CodeGenerator {
        println!("{:?}", &lfc);
        let lfc_path = lfc.unwrap_or(which("lfc").expect("cannot find lingua franca."));

        CodeGenerator {
            lfc: lfc_path,
            properties: LFCProperties::new(src, out, properties),
        }
    }

    pub fn generate_code(self, app: &App) -> std::io::Result<()> {
        // path to the src-gen directory
        let mut src_gen_directory = app.root_path.clone();
        src_gen_directory.push(PathBuf::from("./src-gen"));

        // FIXME: This validation should be somewhere else
        if !self.lfc.exists() {
            panic!("lfc not found at `{}`", &self.lfc.display());
        }

        println!(
            "Invoking code-generator: `{} --json={}`",
            &self.lfc.display(),
            self.properties
        );

        let mut command = Command::new(format!("{}", &self.lfc.display()));
        command.arg(format!("--json={}", self.properties));

        match run_and_capture(&mut command) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("error while generating code {:?}", e);
                Err(e)
            }
        }
    }
}
