pub(crate) mod populator;
use crate::aast::Def;
use crate::aast::Stmt;
use crate::ast_visitors::{Scanner, Context};

use analyzer::analyze_files;

// use ast_visitors::AstJson;
// use ast_visitors::Context;
use diff::{mark_safe_symbols_from_diff, CachedAnalysis};
use file::{FileStatus, VirtualFileSystem};
use hakana_aast_helper::get_aast_for_path_and_contents;

use hakana_analyzer::config::Config;
use hakana_analyzer::dataflow::program_analyzer::{find_connections, find_tainted_data};
use hakana_logger::Logger;
use hakana_reflection_info::analysis_result::AnalysisResult;
use hakana_reflection_info::code_location::{FilePath, HPos};
use hakana_reflection_info::codebase_info::CodebaseInfo;
use hakana_reflection_info::data_flow::graph::{GraphKind, WholeProgramKind};
use hakana_reflection_info::file_info::ParserError;
use hakana_reflection_info::issue::{Issue, IssueKind};
use hakana_reflection_info::symbol_references::SymbolReferences;
use hakana_str::{Interner, StrId};
use indicatif::ProgressBar;

use oxidized::{
    aast,
    aast_visitor::{visit, AstParams, Node, Visitor},
    ast_defs,
};
use oxidized::ast::Hint_;
use oxidized::ast::Stmt_;
use oxidized::scoured_comments::ScouredComments;
use populator::populate_codebase;
use rust_embed::RustEmbed;
use rustc_hash::{FxHashMap, FxHashSet};
use scanner::{scan_files, ScanFilesResult};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use serde_json::{self, json};
use std::fs;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use tower_lsp::lsp_types::MessageType;
use tower_lsp::Client;
use unused_symbols::find_unused_definitions;

mod analyzer;
mod ast_differ;
mod cache;
mod diff;
pub mod file;
pub mod scanner;
mod unused_symbols;
pub mod wasm;
pub mod ast_visitors;
#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../third-party/hhvm/hphp/hack/hhi"]
#[prefix = "hhi_embedded_"]
#[include = "*.hhi"]
#[include = "*.php"]
#[include = "*.hack"]
struct HhiAsset;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../third-party/hhvm/hphp/hsl/src"]
#[prefix = "hsl_embedded_"]
#[include = "*.hhi"]
#[include = "*.php"]
#[include = "*.hack"]
struct HslAsset;

#[derive(Clone, Debug)]
pub struct SuccessfulScanData {
    pub codebase: CodebaseInfo,
    pub interner: Interner,
    pub file_system: VirtualFileSystem,
    pub resolved_names: FxHashMap<FilePath, FxHashMap<u32, StrId>>,
}

impl Default for SuccessfulScanData {
    fn default() -> Self {
        SuccessfulScanData {
            codebase: CodebaseInfo::new(),
            interner: Interner::default(),
            file_system: VirtualFileSystem::default(),
            resolved_names: FxHashMap::default(),
        }
    }
}

pub async fn scan_and_analyze_async(
    stubs_dirs: Vec<String>,
    filter: Option<String>,
    ignored_paths: Option<FxHashSet<String>>,
    config: Arc<Config>,
    threads: u8,
    lsp_client: &Client,
    header: &str,
    previous_scan_data: Option<SuccessfulScanData>,
    previous_analysis_result: Option<AnalysisResult>,
    language_server_changes: Option<FxHashMap<String, FileStatus>>,
) -> io::Result<(AnalysisResult, SuccessfulScanData)> {
    let mut all_scanned_dirs = stubs_dirs.clone();
    all_scanned_dirs.push(config.root_dir.clone());

    lsp_client
        .log_message(MessageType::INFO, "Scanning files")
        .await;

    tokio::time::sleep(Duration::from_millis(10)).await;

    let ScanFilesResult {
        mut codebase,
        mut interner,
        resolved_names,
        codebase_diff,
        file_system,
        mut files_to_analyze,
        invalid_files,
    } = scan_files(
        &all_scanned_dirs,
        None,
        &config,
        threads,
        Arc::new(Logger::DevNull),
        header,
        previous_scan_data,
        language_server_changes,
    )?;

    let mut cached_analysis = if config.ast_diff {
        mark_safe_symbols_from_diff(
            &Arc::new(Logger::DevNull),
            codebase_diff,
            &codebase,
            &mut interner,
            invalid_files,
            &mut files_to_analyze,
            &None,
            &None,
            previous_analysis_result,
        )
    } else {
        CachedAnalysis::default()
    };

    lsp_client
        .log_message(MessageType::INFO, "Calculating symbol inheritance")
        .await;

    tokio::time::sleep(Duration::from_millis(10)).await;

    populate_codebase(
        &mut codebase,
        &interner,
        &mut cached_analysis.symbol_references,
        cached_analysis.safe_symbols,
        cached_analysis.safe_symbol_members,
    );

    let (analysis_result, arc_scan_data) = get_analysis_ready(
        &config,
        codebase,
        interner,
        file_system,
        resolved_names,
        cached_analysis.symbol_references,
        cached_analysis.existing_issues,
    );

    lsp_client
        .log_message(
            MessageType::INFO,
            &format!("Analyzing {} files", files_to_analyze.len()),
        )
        .await;

    tokio::time::sleep(Duration::from_millis(10)).await;

    analyze_files(
        files_to_analyze,
        arc_scan_data.clone(),
        config.clone(),
        &analysis_result,
        filter,
        &ignored_paths,
        threads,
        Arc::new(Logger::DevNull),
        &mut Duration::default(),
    )?;

    let mut analysis_result = (*analysis_result.lock().unwrap()).clone();

    let scan_data = Arc::try_unwrap(arc_scan_data).unwrap();

    add_invalid_files(&scan_data, &mut analysis_result);

    if config.find_unused_definitions {
        find_unused_definitions(
            &mut analysis_result,
            &config,
            &scan_data.codebase,
            &scan_data.interner,
            &ignored_paths,
        );
    }

    Ok((analysis_result, scan_data))
}

pub fn scan_and_analyze(
    stubs_dirs: Vec<String>,
    filter: Option<String>,
    ignored_paths: Option<FxHashSet<String>>,
    config: Arc<Config>,
    cache_dir: Option<&String>,
    threads: u8,
    logger: Arc<Logger>,
    header: &str,
    previous_scan_data: Option<SuccessfulScanData>,
    previous_analysis_result: Option<AnalysisResult>,
    language_server_changes: Option<FxHashMap<String, FileStatus>>,
) -> io::Result<(AnalysisResult, SuccessfulScanData)> {
    let mut all_scanned_dirs = stubs_dirs.clone();
    all_scanned_dirs.push(config.root_dir.clone());

    let file_discovery_and_scanning_now = Instant::now();

    logger.log_sync("Scanning files");

    let ScanFilesResult {
        mut codebase,
        mut interner,
        resolved_names,
        codebase_diff,
        file_system,
        mut files_to_analyze,
        invalid_files,
    } = scan_files(
        &all_scanned_dirs,
        cache_dir,
        &config,
        threads,
        logger.clone(),
        header,
        previous_scan_data,
        language_server_changes,
    )?;

    let file_discovery_and_scanning_elapsed = file_discovery_and_scanning_now.elapsed();

    if logger.can_log_timing() {
        logger.log_sync(&format!(
            "File discovery & scanning took {:.2?}",
            file_discovery_and_scanning_elapsed
        ));
    }

    if let Some(cache_dir) = cache_dir {
        let timestamp_path = format!("{}/buildinfo", cache_dir);
        let mut timestamp_file = fs::File::create(timestamp_path).unwrap();
        write!(timestamp_file, "{}", header).unwrap();

        let aast_manifest_path = format!("{}/manifest", cache_dir);
        fs::File::create(&aast_manifest_path)
            .unwrap()
            .write_all(&bincode::serialize(&file_system).unwrap())
            .unwrap_or_else(|_| panic!("Could not write aast manifest {}", &aast_manifest_path));
    }

    let mut cached_analysis = if config.ast_diff {
        mark_safe_symbols_from_diff(
            &logger,
            codebase_diff,
            &codebase,
            &mut interner,
            invalid_files,
            &mut files_to_analyze,
            &get_issues_path(cache_dir),
            &get_references_path(cache_dir),
            previous_analysis_result,
        )
    } else {
        CachedAnalysis::default()
    };

    logger.log_sync("Calculating symbol inheritance");

    let populating_now = Instant::now();

    populate_codebase(
        &mut codebase,
        &interner,
        &mut cached_analysis.symbol_references,
        cached_analysis.safe_symbols,
        cached_analysis.safe_symbol_members,
    );

    let populating_elapsed = populating_now.elapsed();

    if logger.can_log_timing() {
        logger.log_sync(&format!(
            "Populating codebase took {:.2?}",
            populating_elapsed
        ));
    }

    let (analysis_result, arc_scan_data) = get_analysis_ready(
        &config,
        codebase,
        interner,
        file_system,
        resolved_names,
        cached_analysis.symbol_references,
        cached_analysis.existing_issues,
    );

    logger.log_sync(&format!("Analyzing {} files", files_to_analyze.len()));

    let mut pure_file_analysis_time = Duration::default();

    analyze_files(
        files_to_analyze,
        arc_scan_data.clone(),
        config.clone(),
        &analysis_result,
        filter,
        &ignored_paths,
        threads,
        logger.clone(),
        &mut pure_file_analysis_time,
    )?;

    if logger.can_log_timing() {
        logger.log_sync(&format!(
            "File analysis took {:.2?} (excluding re-parsing)",
            pure_file_analysis_time
        ));
    }

    let mut analysis_result = (*analysis_result.lock().unwrap()).clone();

    analysis_result.time_in_analysis = pure_file_analysis_time;

    cache_analysis_data(cache_dir, &analysis_result)?;

    let scan_data = Arc::try_unwrap(arc_scan_data).unwrap();

    add_invalid_files(&scan_data, &mut analysis_result);

    if config.find_unused_definitions {
        find_unused_definitions(
            &mut analysis_result,
            &config,
            &scan_data.codebase,
            &scan_data.interner,
            &ignored_paths,
        );
    }

    if let GraphKind::WholeProgram(whole_program_kind) = config.graph_kind {
        let issues = match whole_program_kind {
            WholeProgramKind::Taint => find_tainted_data(
                &analysis_result.program_dataflow_graph,
                &config,
                &logger,
                &scan_data.interner,
            ),
            WholeProgramKind::Query => find_connections(
                &analysis_result.program_dataflow_graph,
                &config,
                &logger,
                &scan_data.interner,
            ),
        };

        for issue in issues {
            analysis_result
                .emitted_issues
                .entry(issue.pos.file_path)
                .or_default()
                .push(issue);
        }
    }

    Ok((analysis_result, scan_data))
}

fn get_analysis_ready(
    config: &Arc<Config>,
    codebase: CodebaseInfo,
    interner: Interner,
    file_system: VirtualFileSystem,
    resolved_names: FxHashMap<FilePath, FxHashMap<u32, StrId>>,
    symbol_references: SymbolReferences,
    existing_issues: FxHashMap<FilePath, Vec<Issue>>,
) -> (Arc<Mutex<AnalysisResult>>, Arc<SuccessfulScanData>) {
    let mut analysis_result = AnalysisResult::new(config.graph_kind, symbol_references);

    analysis_result.emitted_issues = existing_issues;

    let analysis_result = Arc::new(Mutex::new(analysis_result));

    let scan_data = SuccessfulScanData {
        codebase,
        interner,
        file_system,
        resolved_names,
    };

    let arc_scan_data = Arc::new(scan_data);
    (analysis_result, arc_scan_data)
}

fn cache_analysis_data(
    cache_dir: Option<&String>,
    analysis_result: &AnalysisResult,
) -> Result<(), io::Error> {
    if let Some(references_path) = get_references_path(cache_dir) {
        let mut symbols_file = fs::File::create(references_path).unwrap();
        let serialized_symbol_references =
            bincode::serialize(&analysis_result.symbol_references).unwrap();
        symbols_file.write_all(&serialized_symbol_references)?;
    }
    if let Some(issues_path) = get_issues_path(cache_dir) {
        let mut issues_file = fs::File::create(issues_path).unwrap();
        let serialized_issues = bincode::serialize(&analysis_result.emitted_issues).unwrap();
        issues_file.write_all(&serialized_issues)?;
    };
    Ok(())
}

fn get_issues_path(cache_dir: Option<&String>) -> Option<String> {
    cache_dir.map(|cache_dir| format!("{}/issues", cache_dir))
}

fn get_references_path(cache_dir: Option<&String>) -> Option<String> {
    cache_dir.map(|cache_dir| format!("{}/references", cache_dir))
}

pub fn get_aast_for_path(
    file_path: FilePath,
    file_path_str: &str,
) -> Result<(aast::Program<(), ()>, ScouredComments, String), ParserError> {
    let file_contents = if file_path_str.starts_with("hsl_embedded_") {
        std::str::from_utf8(
            &HslAsset::get(file_path_str)
                .unwrap_or_else(|| panic!("Could not read HSL file {}", file_path_str))
                .data,
        )
        .unwrap_or_else(|_| panic!("Could not convert HSL file {}", file_path_str))
        .to_string()
    } else if file_path_str.starts_with("hhi_embedded_") {
        std::str::from_utf8(
            &HhiAsset::get(file_path_str)
                .unwrap_or_else(|| panic!("Could not read HSL file {}", file_path_str))
                .data,
        )
        .unwrap_or_else(|_| panic!("Could not convert HHI file {}", file_path_str))
        .to_string()
    } else {
        match fs::read_to_string(file_path_str) {
            Ok(str_file) => str_file,
            Err(err) => {
                eprintln!("Error reading file: {}", err); // Need to do some better error propagation
                return Err(ParserError::CannotReadFile);
            }
        }
    };
    get_aast_for_path_and_contents(file_path, file_path_str, file_contents)
}

#[derive(Serialize, Deserialize, Debug)]
struct Program {
    kind: String,
    children: Vec<Fun>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Fun {
    kind: String,
    doc_comment: String,
    child: FunDef,
}

#[derive(Serialize, Deserialize, Debug)]
struct FunDef {
    pub kind: String,
    pub name: aast::Sid, //change this into a Name Struct Eventually
    pub span: aast::Pos,
    pub params: Vec<FunParam>,
    pub body: FuncBody,
    pub ret: Ret
}

#[derive(Serialize, Deserialize, Debug)]
struct FunParam {
    kind: String,
    name: String,
    type_hint: TypeHint,
    is_variadic: aast::IsVariadic,
}

#[derive(Serialize, Deserialize, Debug)]
struct FuncBody {
    kind: String,
    child: Vec<aast::Stmt<(), ()>>
}
#[derive(Serialize, Deserialize, Debug)]
struct Ret {
    kind: String,
    type_hint: TypeHint
}
#[derive(Serialize, Deserialize, Debug)]
struct TypeHint {
    kind: String,
    value: Hint_
}

pub fn dump_new_aast_for_path(
    file_path_str: &str,
    output_file_str: &str,
    ) -> std::result::Result<(), ParserError> {
    
    let (aast, comments, idk) = match get_aast_for_path(FilePath(StrId::EMPTY), file_path_str) {
        Ok(aast) => aast,
        Err(err) => {
            return Err(err);
        }
    };
    // println!("{:#?}", comments);
    // println!("{}", idk);
    // println!("{}", serde_json::from_string(&aast).unwrap());
    // let aast_str = serde_json::to_string(&aast).unwrap();
    // let aast_json: Program = serde_json::from_str(&aast_str).unwrap();
    // println!("{}", aast_str);
    // println!("{}", len(aast_json));
    // println!("{}", aast_json.to_string());
    // println!("{:#?}", aast_json);
    let mut child_arr: Vec<Fun> = Vec::new();

    let mut scanner = Scanner {
            tree_stack: Vec::<serde_json::Value>::new(),
            tree: String::new(),
        };

    let mut context = Context {};

    visit(&mut scanner, &mut context, &aast).unwrap();
    
    let mut aast_file = fs::File::create(output_file_str).expect("Unable to create file");
    let aast_format = scanner.tree;
    
    aast_file.write_all(aast_format.as_bytes()).expect("Unable to write to file");
    println!("Output saved to: {}", output_file_str);
    Ok(Default::default())
}


pub fn dump_aast_for_path(
    file_path_str: &str,
    output_file_str: &str,
    ) -> std::result::Result<(), ParserError> {
    let (aast, _comments, _idk) = match get_aast_for_path(FilePath(StrId::EMPTY), file_path_str) {
        Ok(aast) => aast,
        Err(err) => {
            return Err(err);
        }
    };
    let mut aast_file = fs::File::create(output_file_str).expect("Unable to create file");
    let aast_format = format!("{:#?}", aast);
    aast_file.write_all(aast_format.as_bytes()).expect("Unable to write to file");
    println!("Output saved to: {}", output_file_str);
    Ok(Default::default())
}


fn update_progressbar(percentage: u64, bar: Option<Arc<ProgressBar>>) {
    if let Some(bar) = bar {
        bar.set_position(percentage);
    }
}

fn add_invalid_files(scan_data: &SuccessfulScanData, analysis_result: &mut AnalysisResult) {
    for (file_path, file_info) in &scan_data.codebase.files {
        if let Some(parser_error) = &file_info.parser_error {
            analysis_result.emitted_issues.insert(
                *file_path,
                vec![match parser_error {
                    ParserError::NotAHackFile => Issue::new(
                        IssueKind::InvalidHackFile,
                        "Invalid Hack file".to_string(),
                        HPos {
                            file_path: *file_path,
                            start_offset: 0,
                            end_offset: 0,
                            start_line: 0,
                            end_line: 0,
                            start_column: 0,
                            end_column: 0,
                        },
                        &None,
                    ),
                    ParserError::CannotReadFile => Issue::new(
                        IssueKind::InvalidHackFile,
                        "Cannot read file".to_string(),
                        HPos {
                            file_path: *file_path,
                            start_offset: 0,
                            end_offset: 0,
                            start_line: 0,
                            end_line: 0,
                            start_column: 0,
                            end_column: 0,
                        },
                        &None,
                    ),
                    ParserError::SyntaxError { message, pos } => {
                        Issue::new(IssueKind::InvalidHackFile, message.clone(), *pos, &None)
                    }
                }],
            );
        }
    }
}
