use std::fs;

use crate::shader::SpirvStore;
use shaderc::{self, CompileOptions, Compiler, ShaderKind};

use super::{GLSL, HLSL, SPV};

impl HLSL {
    pub fn new<'a>(
        file: &str,
        shader_kind: ShaderKind,
        entry_name: &str,
        options: Option<CompileOptions<'a>>,
    ) -> HLSL {
        let source = fs::read_to_string(&file).unwrap();

        let compiler = Compiler::new().unwrap();
        let mut _options = CompileOptions::new().unwrap();
        let mut _options;
        if let Some(o) = options {
            _options = o;
        } else {
            _options = CompileOptions::new().unwrap();
        }
        _options.set_source_language(shaderc::SourceLanguage::HLSL);
        _options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        let binary =
            compiler.compile_into_spirv(&source, shader_kind, &file, entry_name, Some(&_options));
        let assemply = compiler.compile_into_spirv_assembly(&source, shader_kind, &file, entry_name, Some(&_options)).unwrap();
        println!("{:?}", assemply.as_text());
        if let Err(e) = &binary {
            println!("{e}");
        }
        let binary = binary.unwrap();

        let code = binary.as_binary();
        Self {
            code: code.to_vec(),
            entry_name: entry_name.to_string(),
        }
    }
}

impl SpirvStore for HLSL {
    fn code(&self) -> &[u32] {
        &self.code
    }

    fn entry_name(&self) -> &str {
        &self.entry_name
    }
}

impl GLSL {
    pub fn new<'a>(
        file: &str,
        shader_kind: ShaderKind,
        entry_name: &str,
        options: Option<CompileOptions<'a>>,
    ) -> GLSL {
        let source = fs::read_to_string(&file).unwrap();

        let compiler = Compiler::new().unwrap();
        let mut _options = CompileOptions::new().unwrap();
        let mut _options;
        if let Some(o) = options {
            _options = o;
        } else {
            _options = CompileOptions::new().unwrap();
        }
        _options.set_source_language(shaderc::SourceLanguage::GLSL);
        _options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        let binary = compiler
            .compile_into_spirv(&source, shader_kind, &file, entry_name, Some(&_options))
            .unwrap();
        let code = binary.as_binary();
        Self {
            code: code.to_vec(),
            entry_name: entry_name.to_string(),
        }
    }
}

impl SpirvStore for GLSL {
    fn code(&self) -> &[u32] {
        &self.code
    }

    fn entry_name(&self) -> &str {
        &self.entry_name
    }
}

impl SPV {
    pub fn new<'a>(
        file: &str,
        entry_name: &str,
    ) -> SPV {
        let source = fs::read(&file).unwrap();
        let code = source.chunks_exact(4).map(|c| unsafe{*(c.as_ptr() as *const u32).clone()}).collect();

        Self {
            code,
            entry_name: entry_name.to_string(),
        }
    }
}

impl SpirvStore for SPV {
    fn code(&self) -> &[u32] {
        &self.code
    }

    fn entry_name(&self) -> &str {
        &self.entry_name
    }
}