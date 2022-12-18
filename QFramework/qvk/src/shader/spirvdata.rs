
use std::fs;

use shaderc::{self, Compiler, CompileOptions, ShaderKind};

use super::{HLSL, shader::SpirvProvider};

impl HLSL {
    pub fn new<'a>(file: &str, kind: ShaderKind, options: Option<CompileOptions<'a>>) -> HLSL {
        let source = fs::read_to_string(&file).unwrap();

        let compiler = Compiler::new().unwrap();
        let mut _options = CompileOptions::new().unwrap();
        let mut _options;
        if let Some(o) = options{
            _options = o;
        }
        else{
            _options = CompileOptions::new().unwrap();
        }
        _options.set_source_language(shaderc::SourceLanguage::HLSL);
        let binary = compiler.compile_into_spirv(&source, kind, &file, "main", Some(&_options)).unwrap();
        let code = binary.as_binary();
        Self{
            code: code.to_vec(),
        }
    }
}

impl SpirvProvider for HLSL{
    fn code(&self) -> &[u32] {
        &self.code
    }
}