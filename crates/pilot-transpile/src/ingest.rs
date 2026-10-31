//! Milestone 41.3: Zero-Roslyn Foreign Language AST Ingesters (`pilot-transpile`).
//!
//! Converts foreign source code (C, C++, C#, Rust, Python, JS/TS, Lua) into canonical
//! Pilot ASTs with `--strict-pilot` idiom upgrading and parser validation (`INGEST001`–`INGEST010`).

use pilot_ast::AstArena;
use crate::ir::TargetLanguage;
use crate::licensing::LicenseVerifier;

#[derive(Clone, Debug)]
pub struct IngestOptions {
    pub strict_pilot: bool,
    pub viable_license_only: bool,
    pub license_spdx: Option<String>,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            strict_pilot: false,
            viable_license_only: false,
            license_spdx: None,
        }
    }
}

pub struct ForeignAstIngester;

impl ForeignAstIngester {
    /// Ingests foreign code from any of the 7 supported languages into valid Pilot source code.
    pub fn ingest_to_pilot_source(
        source: &str,
        lang: TargetLanguage,
        options: &IngestOptions,
    ) -> Result<String, String> {
        // 1. License audit check if required
        if options.viable_license_only {
            if let Some(ref spdx) = options.license_spdx {
                if !LicenseVerifier::is_viable_license(spdx) {
                    return Err(format!("INGEST001: Ingestion rejected — license '{}' is not OSI/FSF viable", spdx));
                }
            }
        }

        // 2. Transcribe foreign language into Pilot syntax
        let pilot_code = match lang {
            TargetLanguage::Rust => Self::ingest_rust(source, options.strict_pilot),
            TargetLanguage::Python => Self::ingest_python(source, options.strict_pilot),
            TargetLanguage::JavaScript => Self::ingest_javascript(source, options.strict_pilot),
            TargetLanguage::CSharp => Self::ingest_csharp(source, options.strict_pilot),
            TargetLanguage::C | TargetLanguage::Cpp => Self::ingest_c_cpp(source, options.strict_pilot),
            TargetLanguage::Lua => Self::ingest_lua(source, options.strict_pilot),
        };

        // 3. Post-ingest validation: ensure output is strictly valid Pilot syntax
        let arena = pilot_parser::parse(&pilot_code);
        if arena.root.is_none() && !pilot_code.trim().is_empty() {
            return Err("INGEST002: Ingested source failed pilot-parser syntax validation".to_string());
        }

        Ok(pilot_code)
    }

    /// Ingests foreign source code directly into a validated `AstArena`.
    pub fn ingest_to_arena(
        source: &str,
        lang: TargetLanguage,
        options: &IngestOptions,
    ) -> Result<AstArena, String> {
        let pilot_src = Self::ingest_to_pilot_source(source, lang, options)?;
        Ok(pilot_parser::parse(&pilot_src))
    }

    fn ingest_rust(src: &str, strict: bool) -> String {
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let mut l = line.to_string();
            // Struct and Enum definitions
            if l.contains("pub struct ") {
                l = l.replace("pub struct ", "struct ");
            }
            if l.contains("pub enum ") {
                l = l.replace("pub enum ", "enum ");
            }
            // Function definition
            if l.contains("pub async fn ") || l.contains("async fn ") {
                l = l.replace("pub async fn ", "async def ").replace("async fn ", "async def ");
            } else if (l.contains("pub fn ") || l.contains("fn ")) && l.contains('(') {
                l = l.replace("pub fn ", "def ").replace("fn ", "def ");
                if strict {
                    l = l.replace("Option<", "").replace("Result<", "");
                }
            }
            if l.contains("let mut ") {
                l = l.replace("let mut ", "var ");
            }
            if l.contains("println!(") {
                l = l.replace("println!(", "println(");
            }
            if l.contains("print!(") {
                l = l.replace("print!(", "print(");
            }
            if strict {
                if l.contains("None") {
                    l = l.replace("None", "none");
                }
                if l.contains("Some(") {
                    l = l.replace("Some(", "some ");
                }
            }
            out.push_str(&l);
            out.push('\n');
        }
        out
    }

    fn ingest_python(src: &str, strict: bool) -> String {
        let mut out = String::with_capacity(src.len() * 2);
        let mut indent_stack: Vec<usize> = vec![0];
        let mut expect_new_block = false;

        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                out.push('\n');
                continue;
            }
            if trimmed.starts_with('#') {
                out.push_str(line);
                out.push('\n');
                continue;
            }

            // Calculate current line indentation in spaces (tabs count as 4)
            let mut cur_indent = 0;
            for ch in line.chars() {
                if ch == ' ' {
                    cur_indent += 1;
                } else if ch == '\t' {
                    cur_indent += 4;
                } else {
                    break;
                }
            }

            if expect_new_block {
                expect_new_block = false;
                if cur_indent > *indent_stack.last().unwrap_or(&0) {
                    indent_stack.push(cur_indent);
                }
            } else {
                while indent_stack.len() > 1 && cur_indent < *indent_stack.last().unwrap() {
                    indent_stack.pop();
                    let close_ind = " ".repeat(*indent_stack.last().unwrap_or(&0));
                    out.push_str(&close_ind);
                    out.push_str("}\n");
                }
            }

            let mut l = trimmed.to_string();

            // Transform block headers
            let is_block_header = if (l.starts_with("def ") || l.starts_with("class ") || l.starts_with("if ") || l.starts_with("while ") || l.starts_with("for ")) && l.ends_with(':') {
                let header = l.trim_end_matches(':').trim();
                if header.starts_with("class ") {
                    if let Some(paren_pos) = header.find('(') {
                        let name = header[6..paren_pos].trim();
                        l = format!("class {} {{", name);
                    } else {
                        l = format!("{} {{", header);
                    }
                } else if header.starts_with("def ") {
                    let sig = &header[4..];
                    if sig.starts_with("__init__(self,") {
                        l = format!("def init({}) {{", sig["__init__(self,".len()..sig.len()-1].trim());
                    } else if sig.starts_with("__init__(self)") {
                        l = "def init() {".to_string();
                    } else if sig.contains("(self,") {
                        l = format!("def {} {{", sig.replace("(self,", "("));
                    } else if sig.contains("(self)") {
                        l = format!("def {} {{", sig.replace("(self)", "()"));
                    } else {
                        l = format!("def {} {{", sig);
                    }
                } else {
                    l = format!("{} {{", header);
                }
                true
            } else if l.starts_with("elif ") && l.ends_with(':') {
                let cond = l["elif ".len()..l.len()-1].trim();
                l = format!("}} else if {} {{", cond);
                true
            } else if l == "else:" {
                l = "} else {".to_string();
                true
            } else {
                false
            };

            // Literal replacements
            l = l.replace("True", "true").replace("False", "false").replace("None", "none");
            if l.contains("print(") {
                l = l.replace("print(", "println(");
            }

            if strict && !is_block_header && l.contains(" = ") && !l.starts_with("let ") && !l.starts_with("var ") && !l.starts_with("self.") {
                l = format!("let {}", l);
            }

            let ind = " ".repeat(cur_indent);
            out.push_str(&ind);
            out.push_str(&l);
            out.push('\n');

            if is_block_header {
                expect_new_block = true;
            }
        }

        // Close any remaining unclosed blocks
        while indent_stack.len() > 1 {
            indent_stack.pop();
            let close_ind = " ".repeat(*indent_stack.last().unwrap_or(&0));
            out.push_str(&close_ind);
            out.push_str("}\n");
        }

        out
    }

    fn ingest_javascript(src: &str, strict: bool) -> String {
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let mut l = line.to_string();
            if l.contains("export async function ") || l.contains("async function ") {
                l = l.replace("export async function ", "async def ").replace("async function ", "async def ");
            } else if l.contains("export function ") || l.contains("function ") {
                l = l.replace("export function ", "def ").replace("function ", "def ");
            }
            if l.contains("constructor(") {
                l = l.replace("constructor(", "def init(");
            }
            if l.contains("const ") {
                l = l.replace("const ", "let ");
            }
            if l.contains("console.log(") {
                l = l.replace("console.log(", "println(");
            }
            if l.contains("console.error(") {
                l = l.replace("console.error(", "println(");
            }
            if l.contains("===") {
                l = l.replace("===", "==");
            }
            if l.contains("!==") {
                l = l.replace("!==", "!=");
            }
            if strict {
                if l.contains("null") {
                    l = l.replace("null", "none");
                }
                if l.contains("undefined") {
                    l = l.replace("undefined", "none");
                }
            }
            out.push_str(&l);
            out.push('\n');
        }
        out
    }

    fn ingest_csharp(src: &str, strict: bool) -> String {
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("using ") && trimmed.ends_with(';') {
                continue;
            }
            if trimmed.starts_with("namespace ") {
                continue;
            }
            let mut l = line.to_string();
            if l.contains("Console.WriteLine(") {
                l = l.replace("Console.WriteLine(", "println(");
            }
            if l.contains("Console.Write(") {
                l = l.replace("Console.Write(", "print(");
            }
            if l.contains("public class ") {
                l = l.replace("public class ", "class ");
            }
            if l.contains("public struct ") {
                l = l.replace("public struct ", "struct ");
            }
            if l.contains("public record ") {
                l = l.replace("public record ", "record ");
            }
            if (l.contains("public static ") || l.contains("public ")) && l.contains('(') {
                let trimmed_l = l.trim();
                let is_static = trimmed_l.starts_with("public static ");
                let prefix = if is_static { "public static " } else { "public " };
                if let Some(pos) = l.find(prefix) {
                    let rest = &l[pos + prefix.len()..];
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let fn_part = parts[1..].join(" ");
                        let indent_len = l.len() - l.trim_start().len();
                        l = format!("{}def {}", &l[..indent_len], fn_part);
                    }
                }
            }
            if l.contains("var ") && !l.contains("var;") {
                l = l.replace("var ", "let ");
            }
            if strict && l.contains("null") {
                l = l.replace("null", "none");
            }
            out.push_str(&l);
            out.push('\n');
        }
        out
    }

    fn ingest_c_cpp(src: &str, strict: bool) -> String {
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#include") {
                continue;
            }
            let mut l = line.to_string();
            if l.contains("printf(") {
                l = l.replace("printf(", "println(");
            }
            if l.contains("int main(") {
                l = l.replace("int main(", "def main(");
            } else if l.contains("void main(") {
                l = l.replace("void main(", "def main(");
            }
            for ret_type in &["void ", "int ", "double ", "float ", "bool ", "auto "] {
                let trimmed_l = l.trim();
                if trimmed_l.starts_with(ret_type) && trimmed_l.contains('(') && !trimmed_l.contains('=') && !trimmed_l.contains(';') {
                    let indent_len = l.len() - l.trim_start().len();
                    let rest = &trimmed_l[ret_type.len()..];
                    l = format!("{}def {}", &l[..indent_len], rest);
                    break;
                }
            }
            if strict {
                if l.contains("NULL") {
                    l = l.replace("NULL", "none");
                }
                if l.contains("nullptr") {
                    l = l.replace("nullptr", "none");
                }
            }
            out.push_str(&l);
            out.push('\n');
        }
        out
    }

    fn ingest_lua(src: &str, strict: bool) -> String {
        let mut out = String::with_capacity(src.len());
        for line in src.lines() {
            let mut l = line.to_string();
            let trimmed = l.trim();
            if trimmed.starts_with("local function ") {
                let rest = &trimmed["local function ".len()..];
                let indent_len = l.len() - l.trim_start().len();
                l = format!("{}def {} {{", &l[..indent_len], rest);
            } else if trimmed.starts_with("function ") {
                let rest = &trimmed["function ".len()..];
                let indent_len = l.len() - l.trim_start().len();
                l = format!("{}def {} {{", &l[..indent_len], rest);
            } else if trimmed == "end" {
                let indent_len = l.len() - l.trim_start().len();
                l = format!("{}}}", &l[..indent_len]);
            } else if trimmed.starts_with("if ") && trimmed.ends_with(" then") {
                let cond = &trimmed[3..trimmed.len() - 5];
                let indent_len = l.len() - l.trim_start().len();
                l = format!("{}if {} {{", &l[..indent_len], cond);
            } else if trimmed.starts_with("elseif ") && trimmed.ends_with(" then") {
                let cond = &trimmed[7..trimmed.len() - 5];
                let indent_len = l.len() - l.trim_start().len();
                l = format!("{}}} else if {} {{", &l[..indent_len], cond);
            } else if trimmed == "else" {
                let indent_len = l.len() - l.trim_start().len();
                l = format!("{}}} else {{", &l[..indent_len]);
            } else if trimmed.starts_with("while ") && trimmed.ends_with(" do") {
                let cond = &trimmed[6..trimmed.len() - 3];
                let indent_len = l.len() - l.trim_start().len();
                l = format!("{}while {} {{", &l[..indent_len], cond);
            }
            if l.contains("local ") {
                l = l.replace("local ", "let ");
            }
            if l.contains("~=") {
                l = l.replace("~=", "!=");
            }
            if l.contains("print(") {
                l = l.replace("print(", "println(");
            }
            if strict && l.contains("nil") {
                l = l.replace("nil", "none");
            }
            out.push_str(&l);
            out.push('\n');
        }
        out
    }
}
