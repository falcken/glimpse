use std::sync::atomic::AtomicBool;
use std::{fs, thread};
use std::hash::{Hash, DefaultHasher, Hasher};
use std::process::Command;
use std::sync::{Mutex, Arc};
use futures::channel::oneshot;
use regex::Regex;
use tauri::{AppHandle, Manager};  
use std::collections::{HashMap};
use std::future::{Future};
use futures::future::{Shared, FutureExt};
use std::time::{Duration};

pub enum SvgResult {
    Perfect { svg: String },
    Alright { svg: String, errors: Vec<String> },
    Bad { errors: Vec<String> }
}

pub trait LatexMathCompiler {
    fn set_preamble(&self, content: String) -> Result<(), String>;
    fn math_to_svg(&self, math: &String, display_mode: bool) -> impl Future<Output = SvgResult>;
}

#[derive(Clone)]
enum LatexError {
    RenderError { block_idx: usize, line: usize, msg: String },
    Misc(String)
}

#[derive(Clone)]
enum DvisvgmError {
    Misc(String)
}

#[derive(Clone)]
struct RenderResult {
    math_blocks: Vec<MathBlock>, // the blocks that were rendered
    latex_errors: Vec<LatexError>,
    dvisvgm_errors: Vec<DvisvgmError>
}

type SharedFuture<T> = Shared<oneshot::Receiver<T>>;

struct RenderJob {
    pub future: Shared<oneshot::Receiver<RenderResult>>,
    pub math_blocks: Arc<Mutex<Vec<MathBlock>>>,
    finished: Arc<AtomicBool>
}

impl RenderJob {
    pub fn new(initial_vec: Vec<MathBlock>, preamble: String, base_path: std::path::PathBuf) -> Self {
        let (tx, rx) = oneshot::channel::<RenderResult>();

        let shared_rx = rx.shared();

        let math_blocks = Arc::new(Mutex::new(initial_vec));
        let finished = Arc::new(AtomicBool::new(false));

        let math_blocks2 = math_blocks.clone();
        let finished2 = finished.clone();

        tokio::spawn(async move {
            thread::sleep(Duration::from_millis(100));

            let math_blocks = math_blocks2.lock().unwrap();
            finished2.store(true, std::sync::atomic::Ordering::Relaxed);

            println!("Rendering {} math blocks", math_blocks.len());
            tx.send(compile(math_blocks.clone(), false, &preamble, &base_path))
        });

        return RenderJob {
            future: shared_rx,
            math_blocks: math_blocks,
            finished: finished
        }
    }
}

pub struct LatexMathCompilerImpl {
    preamble: Mutex<String>,
    current_renders: Arc<Mutex<HashMap<MathBlock, SharedFuture<RenderResult>>>>,
    base_path: std::path::PathBuf,
    jobs: Mutex<Vec<RenderJob>>,
}

const DEFAULT_PREAMBLE: &str = r#"
    \usepackage{amsmath}
    \usepackage{amssymb}
    \usepackage{amsfonts}
"#;

pub fn read_preamble(app: &AppHandle) -> String {
    match app.path().app_config_dir() {
        Ok(config_dir) => {
            let preamble_path = config_dir.join("preamble.tex");

            if preamble_path.exists() {
                match fs::read_to_string(preamble_path) {
                    Ok(content) => return content, // Found user preamble
                    Err(_) => println!("Error reading preamble file"),
                }
            }
        }
        Err(e) => println!("Could not resolve app config dir: {}", e),
    }

    // Fallback
    DEFAULT_PREAMBLE.to_string()
}


static ID_COUNTER: Mutex<i32> = Mutex::new(0);
fn get_fresh_basename() -> String {
    let mut guard = ID_COUNTER.lock().unwrap();
    let res = guard.clone();
    *guard = res + 1;
    res.to_string()
}

fn hash<T: Hash>(t: &T) -> String {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    let i = s.finish();
    i.to_string()
}

fn hash_math(math: &String, preamble_hash: &String, display_mode: bool) -> String {
    let str = format!("{}-{}-{}", math, preamble_hash, if display_mode { 1 } else { 0 });
    hash(&str)
}

fn create_tex_file(
    tex_dir: &std::path::PathBuf,
    basename: &String,
    contents: &String,
) -> Result<(), String> {
    let tex_path = tex_dir.join(format!("{}.tex", basename));
    std::fs::write(&tex_path, contents).map_err(|e| e.to_string())
}

fn parse_latex_output(
    output: &String,
    ranges: &Vec<(usize, usize)>
) -> Vec<LatexError> {
    // this regex seems to work: ^(?:\Q! LaTeX Error: \E|\!\W)(.*)$(?:.|\n)*?^l\.(\d*)\W
    let regex = Regex::new(r"(?m)^(?:\!\ LaTeX\ Error:\ |\!\W)(.*)$(?:.|\n)*?^l\.(\d*)\W").unwrap();

    let get_block_idx = |line: usize| ranges.iter().position(|&(f, t)| f <= line && line <= t);

    let mut results = vec![];
    for (error, [msg, line]) in regex.captures_iter(output.as_str()).map(|c| c.extract()) {
        let abs_line = line.parse::<usize>().unwrap_or_else(|_| todo!());
        if let Some(block_idx) = get_block_idx(abs_line) {
            let from = ranges[block_idx].0;
            let line = abs_line - from;
            results.push(LatexError::RenderError { block_idx, line, msg: msg.to_string() });
        } else {
            results.push(LatexError::Misc(error.to_string()));
        }
    }
    results
}

// this runs the `latex` compiler on ${tex_dir}/${basename}.tex with output directory ${tex_dir}
fn run_latex_compiler(
    tex_dir: &std::path::PathBuf,
    basename: &String,
    ranges: &Vec<(usize, usize)>,
) -> Vec<LatexError> {
    let tex = tex_dir.join(format!("{}.tex", basename));
    let output = Command::new("latex")
        .args([
            "-interaction=nonstopmode",
            "-output-directory",
            tex_dir.to_str().unwrap(),
            tex.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("`latex` command failed: {}", e));

    match output {
        Ok(output) if output.status.success() => 
            vec![],
        Ok(output) => 
            parse_latex_output(&String::from_utf8_lossy(&output.stdout).to_string(), ranges),
        Err(error) => 
            vec![ LatexError::Misc(error.to_string()) ]
    }
}

fn run_dvisvgm(
    tex_dir: &std::path::PathBuf,
    svg_dir: &std::path::PathBuf,
    basename: &String,
) -> Vec<DvisvgmError> {
    let dvi_path = tex_dir.join(format!("{}.dvi", basename));
    let output_pattern = svg_dir.join(format!("{}-%3p.svg", basename));
    let output = Command::new("dvisvgm")
        .args([
            "--zoom=1.1", // Seems to fix scaling issues
            "--exact-bbox",
            "--no-fonts",
            "--page=-",
            format!("--output={}", output_pattern.to_str().unwrap()).as_str(),
            dvi_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("`dvisvgm` command failed: {}", e));

    match output {
        Ok(output) if output.status.success() => 
            vec![], // no errors
        Ok(output) =>
            vec![ DvisvgmError::Misc(format!("dvisvgm conversion failed: {}", String::from_utf8_lossy(&output.stderr))) ],
        Err(err) => 
            vec![ DvisvgmError::Misc(err) ]
    }
}

fn get_svg_name(math: &String, display_mode: bool, preamble_hash: &String, has_errors: bool) -> String {
    if has_errors {
        format!("{}.svg", hash_math(math, preamble_hash, display_mode))
    } else {
        format!("errored-{}.svg", hash_math(math, preamble_hash, display_mode))
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct MathBlock {
    display_mode: bool,
    math: String,
}

fn rename_svgs(
    math_blocks: &Vec<MathBlock>,
    have_errors: &Vec<bool>,
    svg_dir: &std::path::PathBuf,
    basename: &String,
    preamble_hash: &String,
) -> Result<(), String> {
    assert_eq!(math_blocks.len(), have_errors.len());
    // rename output svgs to be hash of input
    for i in 0..math_blocks.len() {
        let old_name = format!("{}-{:0>3}.svg", basename, (i + 1));
        let new_name = get_svg_name(&math_blocks[i].math, math_blocks[i].display_mode, preamble_hash, have_errors[i]);

        let [old_path, new_path] = [&old_name, &new_name].map(|n| svg_dir.join(n));

        match std::fs::rename(&old_path, &new_path) {
            Err(err) => return Err(format!("Renaming {} to {} failed:\n {}", &old_name, &new_name, err.to_string())),
            _ => ()
        }
    }

    Ok(())
}

fn remove_scratch_files(
    tex_dir: &std::path::PathBuf,
    basename: &String,
) -> Result<(), String> {
    for f in vec![
        format!("{}.tex", &basename),
        format!("{}.aux", &basename),
        format!("{}.dvi", &basename),
        format!("{}.log", &basename),
    ] {
        if std::fs::remove_file(tex_dir.join(&f)).is_err() {
            // TODO: handle this case
        }
    }

    Ok(())
}

fn compile(
    math_blocks: Vec<MathBlock>,
    display_mode: bool,
    preamble_content: &str,
    base_path: &std::path::PathBuf,
) -> RenderResult {

    let tex_dir = base_path.join("tex");
    let svg_dir = base_path.join("svg");
    let basename = get_fresh_basename();
    let preamble_hash = hash(&preamble_content.to_string());

    let LatexContent { content, ranges } = generate_latex_content(&math_blocks, display_mode, preamble_content);

    // todo: errors
    create_tex_file(&tex_dir, &basename, &content);
    let latex_errors = run_latex_compiler(&tex_dir, &basename, &ranges);
    let dvisvgm_errors = run_dvisvgm(&tex_dir, &svg_dir, &basename);

    let error_pertaining_to = |i: usize| {
        latex_errors.iter().find(|e| match e {
            LatexError::RenderError { block_idx, line, msg } => *block_idx == i,
            LatexError::Misc(_) => true,
        }).is_some() 
        ||
        dvisvgm_errors.iter().find(|e| match e {
            DvisvgmError::Misc(_) => true,
        }).is_some()
    };

    let have_errors = (0..math_blocks.len()).map(|i| error_pertaining_to(i)).collect::<Vec<bool>>();
    rename_svgs(&math_blocks, &have_errors, &svg_dir, &basename, &preamble_hash);

    if remove_scratch_files(&tex_dir, &basename).is_err() { todo!("handle cleaning failed") }

    RenderResult { math_blocks, latex_errors, dvisvgm_errors }
}

fn create_equation_page(math: &str, display_mode: bool) -> String {
    if display_mode {
        format!("\\begin{{page}}\\begin{{equation*}}\\rlap{{\\textcolor[RGB]{{244, 244, 244}}{{\\rule[0pt]{{1pt}}{{1pt}}}}}}\n{}\n\\end{{equation*}}\n\\end{{page}}", math)
    } else {
        format!("\\begin{{page}}$\\rlap{{\\textcolor[RGB]{{244, 244, 244}}{{\\rule[0pt]{{1pt}}{{1pt}}}}}}{}$\n\\end{{page}}", math)
    }
}

// TODO: handle display mode
struct LatexContent {
    content: String,
    ranges: Vec<(usize, usize)>, // list of ranges corresponding to input math blocks
}

fn generate_latex_content(math_blocks: &Vec<MathBlock>, display_mode: bool, preamble_content: &str) -> LatexContent {
    let content = format!(
        r#"
            \documentclass[dvisvgm, preview, 12pt, multi=page]{{standalone}}
            \usepackage[utf8]{{inputenc}}
            \usepackage{{xcolor}}
            % --- Preamble below ---
            {}
            % --- Input below ---
            \begin{{document}}
    "#, preamble_content);

    let mut result = LatexContent {
        content, ranges: vec![]
    };

    let str_lines = |s: &String| s.as_bytes().iter().filter(|&c| *c == ('\n' as u8)).count();

    for MathBlock { math, display_mode } in math_blocks.iter() {
        let from = str_lines(&result.content) + 1;
        let page = create_equation_page(math, *display_mode);
        let to = from + str_lines(&page);
        result.content += &page;
        result.ranges.push((from, to));
    }

    result.content += "\n\\end{document}";

    return result
}

fn svg_lookup(svg_dir: &std::path::PathBuf, math: &String, display_mode: bool, preamble_hash: &String, has_errors: bool) -> Option<String> {
    let svg_path = svg_dir.join(get_svg_name(math, display_mode, preamble_hash, has_errors));

    match std::fs::exists(&svg_path) {
        Ok(false) => return None,
        Ok(true) => (),
        Err(_) => todo!("handle exists failure in cache lookup"),
    }

    match std::fs::read_to_string(svg_path) {
        Ok(svg_content) => Some(svg_content),
        Err(_) => todo!("handle read failure in cache lookup")
    }
}

impl LatexMathCompilerImpl {
    pub fn new(base_path: std::path::PathBuf, initial_preamble: String) -> Self {
        return Self {
            preamble: Mutex::new(initial_preamble),
            current_renders: Arc::new(Mutex::new(HashMap::new())),
            base_path: base_path,
            jobs: Mutex::new(Vec::new())
        }
    }

    fn render_at_some_point(&self, block: MathBlock) -> SharedFuture<RenderResult> {
        let mut jobs = self.jobs.lock().unwrap();
        for i in 0 .. jobs.len() {
            let job = jobs.get(i).unwrap();

            // aquire lock eagerly
            let mut math_blocks = job.math_blocks.lock().unwrap();

            if math_blocks.len() < 10 && !job.finished.load(std::sync::atomic::Ordering::Relaxed) {
                math_blocks.push(block);
                return job.future.clone();
            }
        };

        jobs.retain(|job| !job.finished.load(std::sync::atomic::Ordering::Relaxed));

        jobs.push(RenderJob::new(vec![ block ], self.preamble.lock().unwrap().clone(), self.base_path.clone()));

        jobs.last().unwrap().future.clone()
    }

    async fn obtain_render_future(&self, block: &MathBlock) -> RenderResult {
        let (future, owner) = {
            let mut future_map = self.current_renders.lock().unwrap();
            if let Some(shared_future) = future_map.get(&block) {
                // this exact math is currently being rendered
                (shared_future.clone(), false)
            } else {
                let future = self.render_at_some_point(block.clone());
                future_map.insert(block.clone(), future.clone());
                (future, true)
            }
        };
        let result = future.await.unwrap_or_else(|_| todo!("Future somehow failed..."));
        if owner {
            self.current_renders.lock().unwrap()
                .remove(block);
        }
        result
    }
}

fn filter_errors(result: RenderResult, math: &String, display_mode: bool) -> Vec<String> {
    if let Some(idx) = result.math_blocks.iter().position(|m| m == &MathBlock { math: math.clone(), display_mode }) {
        let mut errors = result.dvisvgm_errors.iter().map(|e| match e {
            DvisvgmError::Misc(str) => str.clone(),
        }).collect::<Vec<String>>();
        let mut errors2 = result.latex_errors.iter().flat_map(|e| match e {
            LatexError::RenderError { block_idx, line, msg } if *block_idx == idx =>
                Some(format!("{}: {}", line, msg)),
            LatexError::RenderError { block_idx: _, line: _, msg: _ } => None,
            LatexError::Misc(str) => Some(str.clone()),
        }).collect::<Vec<String>>();
        errors.append(&mut errors2);
        errors
    } else {
        todo!()
    }
}

impl LatexMathCompiler for LatexMathCompilerImpl {
    fn set_preamble(&self, content: String) -> Result<(), String> {
        { // set preamble
            let mut preamble = self.preamble.lock().unwrap();
            *preamble = content;
        }
        Ok(())
    }

    async fn math_to_svg(&self, math: &String, display_mode: bool) -> SvgResult {
        let svg_dir = &self.base_path.join("svg");

        let preamble_hash = hash(&self.preamble.lock().unwrap().clone());
        if let Some(svg) = svg_lookup(&svg_dir, &math, display_mode, &preamble_hash, false) {
            return SvgResult::Perfect { svg }
        }

        let block = MathBlock { display_mode, math: math.clone() };
        let result = self.obtain_render_future(&block).await;
        match filter_errors(result, &math, display_mode) {
            errors if errors.is_empty() => SvgResult::Perfect { 
                svg: svg_lookup(&svg_dir, &math, display_mode, &preamble_hash, false)
                    .unwrap_or_else(|| todo!("BUG: no svg after perfect render"))
            },
            errors => match svg_lookup(&svg_dir, &math, display_mode, &preamble_hash, true) {
                Some(svg) => SvgResult::Alright { svg, errors },
                None => SvgResult::Bad { errors },
            }
        }
    }
}
