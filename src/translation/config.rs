use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::Context;
use pulldown_cmark::{Parser, html, Options};

use super::backend::BackendConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    PlainText,
    Markdown,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::PlainText
    }
}

/// 将Markdown文本转换为HTML，同时保留LaTeX公式
pub fn markdown_to_html(markdown: &str) -> String {
    // 先保存LaTeX公式，避免被Markdown解析器修改
    let (protected_text, placeholders) = protect_latex_formulas(markdown);
    
    // 启用所有Markdown扩展选项
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    
    // 解析并转换Markdown
    let parser = Parser::new_ext(&protected_text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    // 还原LaTeX公式
    restore_latex_formulas(&html_output, &placeholders)
}

/// 保护LaTeX公式，替换为占位符
fn protect_latex_formulas(text: &str) -> (String, HashMap<String, String>) {
    let mut result = text.to_string();
    let mut placeholders = HashMap::new();
    let mut placeholder_id = 0;
    
    // 处理行内公式 $...$
    protect_formula_pattern(&mut result, &mut placeholders, &mut placeholder_id, r"\$([^\$]+?)\$", false);
    
    // 处理行内公式 \(...\)
    protect_formula_pattern(&mut result, &mut placeholders, &mut placeholder_id, r"\\\((.+?)\\\)", false);
    
    // 处理行间公式 $$...$$
    protect_formula_pattern(&mut result, &mut placeholders, &mut placeholder_id, r"\$\$([^\$]+?)\$\$", true);
    
    // 处理行间公式 \[...\]
    protect_formula_pattern(&mut result, &mut placeholders, &mut placeholder_id, r"\\\[(.+?)\\\]", true);
    
    (result, placeholders)
}

/// 针对特定模式的公式进行保护
fn protect_formula_pattern(
    text: &mut String, 
    placeholders: &mut HashMap<String, String>, 
    id: &mut i32,
    pattern: &str,
    display_mode: bool
) {
    let re = regex::Regex::new(pattern).unwrap();
    
    while let Some(cap) = re.captures(text) {
        let formula = cap[1].to_string();
        let placeholder = format!("LATEX_FORMULA_{}", id);
        *id += 1;
        
        // 添加到占位符映射中，标记是否是行间公式
        placeholders.insert(placeholder.clone(), if display_mode {
            format!("DISPLAY:{}", formula)
        } else {
            formula
        });
        
        // 替换为占位符
        *text = text.replacen(&cap[0], &format!("{{{}}}", placeholder), 1);
    }
}

/// 从HTML中还原LaTeX公式
fn restore_latex_formulas(html: &str, placeholders: &HashMap<String, String>) -> String {
    let mut result = html.to_string();
    
    for (placeholder, formula) in placeholders {
        let placeholder_pattern = format!("{{{}}}", placeholder);
        
        if formula.starts_with("DISPLAY:") {
            // 行间公式
            let actual_formula = &formula[8..];
            let rendered = format!(
                r#"<span class="katex-display"><span class="katex-math">\[{}\]</span></span>"#, 
                actual_formula
            );
            result = result.replace(&placeholder_pattern, &rendered);
        } else {
            // 行内公式
            let rendered = format!(
                r#"<span class="katex-math">\({}\)</span>"#, 
                formula
            );
            result = result.replace(&placeholder_pattern, &rendered);
        }
    }
    
    result
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_backend: Option<String>,
    pub backends: HashMap<String, BackendConfig>,
    #[serde(default)]
    pub prompts: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut prompts = HashMap::new();
        prompts.insert(
            "translation".to_string(),
            r#"You are a professional translator. 
Translate the following text accurately while preserving the original meaning and style. 
Only return the translated text without any explanations or additional content.

Text to translate: {text}
Source language: {source}
Target language: {target}"#.to_string(),
        );

        Self {
            default_backend: None,
            backends: HashMap::new(),
            prompts,
        }
    }
}

impl Config {
    pub fn config_path() -> anyhow::Result<PathBuf> {
        let current_dir = env::current_dir()
            .context("Failed to get current directory")?;
        Ok(current_dir.join("rs_translator_config.json"))
    }

    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            let config = serde_json::from_str(&content)
                .context("Failed to parse config JSON")?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        fs::write(config_path, content)
            .context("Failed to write config file")?;
        Ok(())
    }

    pub fn get_prompt(&self, key: &str) -> Option<&str> {
        self.prompts.get(key).map(|s| s.as_str())
    }
}
