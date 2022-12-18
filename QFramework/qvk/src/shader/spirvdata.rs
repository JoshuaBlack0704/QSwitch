
use std::fs;

use shaderc::{self, Compiler, CompileOptions, ShaderKind};

use super::{HLSL, shader::SpirvProvider};

impl HLSL {
    pub fn new<'a>(file: &str, shader_kind: ShaderKind, entry_name: &str, options: Option<CompileOptions<'a>>) -> HLSL {
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
        let binary = compiler.compile_into_spirv(&source, shader_kind, &file, entry_name, Some(&_options)).unwrap();
        let code = binary.as_binary();
        Self{
            code: code.to_vec(),
            entry_name: entry_name.to_string(),
        }
    }
}

impl SpirvProvider for HLSL{
    fn code(&self) -> &[u32] {
        &self.code
    }

    fn entry_name(&self) -> &str {
        &self.entry_name
    }
}