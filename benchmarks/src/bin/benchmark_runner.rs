//! Chronicle Benchmark Runner
//!
//! Main binary for running Chronicle benchmarks with various options and configurations.

use anyhow::Result;
use chronicle_benchmarks::{
    config::Config, init, run_all_benchmarks, run_benchmark, BenchmarkComponent, BenchmarkConfig,
    PerformanceTargets,
};
use clap::{Parser, Subcommand};
use std::time::Duration;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "benchmark-runner")]
#[command(about = "Chronicle Performance Benchmark Runner")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long, default_value = "benchmark_config.toml")]
    config: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output format
    #[arg(short, long, default_value = "json")]
    output: String,

    /// Output file
    #[arg(short, long)]
    file: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all benchmarks
    All {
        /// Duration in seconds
        #[arg(short, long, default_value = "10")]
        duration: u64,
        
        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: u32,
        
        /// Concurrency level
        #[arg(short, long, default_value = "1")]
        concurrency: u32,
    },
    /// Run specific component benchmark
    Component {
        /// Component to benchmark
        #[arg(value_enum)]
        component: ComponentArg,
        
        /// Specific test name (optional)
        #[arg(short, long)]
        test: Option<String>,
        
        /// Duration in seconds
        #[arg(short, long, default_value = "10")]
        duration: u64,
        
        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: u32,
        
        /// Concurrency level
        #[arg(short, long, default_value = "1")]
        concurrency: u32,
    },
    /// List available benchmarks
    List,
    /// Validate benchmark configuration
    Validate,
    /// Generate sample configuration
    Config,
}

#[derive(clap::ValueEnum, Clone)]
enum ComponentArg {
    RingBuffer,
    Collectors,
    Packer,
    Search,
    Storage,
    System,
}

impl From<ComponentArg> for BenchmarkComponent {
    fn from(arg: ComponentArg) -> Self {
        match arg {
            ComponentArg::RingBuffer => BenchmarkComponent::RingBuffer,
            ComponentArg::Collectors => BenchmarkComponent::Collectors,
            ComponentArg::Packer => BenchmarkComponent::Packer,
            ComponentArg::Search => BenchmarkComponent::Search,
            ComponentArg::Storage => BenchmarkComponent::Storage,
            ComponentArg::System => BenchmarkComponent::System,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    // Initialize benchmarking system
    init()?;

    match cli.command {
        Commands::All { duration, iterations, concurrency } => {
            run_all_benchmarks_command(duration, iterations, concurrency, &cli).await?;
        }
        Commands::Component { component, test, duration, iterations, concurrency } => {
            run_component_benchmark_command(
                component.into(),
                test,
                duration,
                iterations,
                concurrency,
                &cli,
            ).await?;
        }
        Commands::List => {
            list_benchmarks_command().await?;
        }
        Commands::Validate => {
            validate_config_command(&cli).await?;
        }
        Commands::Config => {
            generate_config_command().await?;
        }
    }

    Ok(())
}

async fn run_all_benchmarks_command(
    duration: u64,
    iterations: u32,
    concurrency: u32,
    cli: &Cli,
) -> Result<()> {
    info!("Running all Chronicle benchmarks...");

    let config = BenchmarkConfig {
        duration: Duration::from_secs(duration),
        warmup_duration: Duration::from_secs(2),
        iterations,
        concurrency,
        data_size: 1024 * 1024, // 1MB
        targets: PerformanceTargets::default(),
    };

    let start_time = std::time::Instant::now();
    let results = run_all_benchmarks(&config).await?;
    let total_duration = start_time.elapsed();

    info!(
        "Completed {} benchmarks in {:.2?}",
        results.len(),
        total_duration
    );

    // Print summary
    let mut passed = 0;
    let mut failed = 0;
    
    for result in &results {
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
            warn!(
                "Benchmark {} failed: {}",
                result.test_name,
                result.notes.as_deref().unwrap_or("Unknown error")
            );
        }
    }

    info!("Summary: {} passed, {} failed", passed, failed);

    // Output results
    output_results(&results, &cli.output, cli.file.as_deref()).await?;

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

async fn run_component_benchmark_command(
    component: BenchmarkComponent,
    test_name: Option<String>,
    duration: u64,
    iterations: u32,
    concurrency: u32,
    cli: &Cli,
) -> Result<()> {
    let config = BenchmarkConfig {
        duration: Duration::from_secs(duration),
        warmup_duration: Duration::from_secs(2),
        iterations,
        concurrency,
        data_size: 1024 * 1024, // 1MB
        targets: PerformanceTargets::default(),
    };

    let results = if let Some(test) = test_name {
        info!("Running {} benchmark: {}", component, test);
        vec![run_benchmark(component, &test, &config).await?]
    } else {
        info!("Running all {} benchmarks", component);
        match component {
            BenchmarkComponent::RingBuffer => {
                chronicle_benchmarks::benches::ring_buffer_bench::run_all_benchmarks(&config).await?
            }
            BenchmarkComponent::Collectors => {
                chronicle_benchmarks::benches::collectors_bench::run_all_benchmarks(&config).await?
            }
            BenchmarkComponent::Packer => {
                chronicle_benchmarks::benches::packer_bench::run_all_benchmarks(&config).await?
            }
            BenchmarkComponent::Search => {
                chronicle_benchmarks::benches::search_bench::run_all_benchmarks(&config).await?
            }
            BenchmarkComponent::Storage => {
                chronicle_benchmarks::benches::storage_bench::run_all_benchmarks(&config).await?
            }
            BenchmarkComponent::System => {
                chronicle_benchmarks::monitoring::system_monitor::run_all_benchmarks(&config).await?
            }
        }
    };

    // Print results
    for result in &results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        info!(
            "[{}] {}: {:.2} events/s, {:.2} MB/s, {:.2}ms p95",
            status,
            result.test_name,
            result.metrics.throughput.events_per_second,
            result.metrics.throughput.bytes_per_second / (1024.0 * 1024.0),
            result.metrics.latency.p95_ms
        );
    }

    // Output results
    output_results(&results, &cli.output, cli.file.as_deref()).await?;

    Ok(())
}

async fn list_benchmarks_command() -> Result<()> {
    info!("Available Chronicle benchmarks:");

    let benchmarks = vec![
        ("ring_buffer", vec![
            "single_producer_single_consumer",
            "single_producer_multi_consumer",
            "multi_producer_single_consumer",
            "multi_producer_multi_consumer",
            "burst_write_performance",
            "sustained_throughput",
            "memory_pressure",
            "lock_contention",
        ]),
        ("collectors", vec![
            "keyboard_collector_performance",
            "mouse_collector_performance",
            "window_collector_performance",
            "filesystem_collector_performance",
            "network_collector_performance",
            "audio_collector_performance",
            "screen_collector_performance",
            "all_collectors_combined",
        ]),
        ("packer", vec![
            "data_processing_throughput",
            "compression_performance",
            "encryption_overhead",
            "parquet_write_performance",
            "heif_processing_performance",
            "batch_processing_efficiency",
        ]),
        ("search", vec![
            "simple_text_search",
            "wildcard_search",
            "regex_search",
            "date_range_search",
            "complex_query_search",
            "large_result_set_search",
        ]),
        ("storage", vec![
            "file_write_performance",
            "file_read_performance",
            "database_insert_performance",
            "database_query_performance",
            "compression_efficiency",
            "storage_overhead",
        ]),
        ("system", vec![
            "cpu_monitoring_accuracy",
            "memory_monitoring_accuracy",
            "disk_io_monitoring",
            "network_io_monitoring",
            "process_monitoring",
            "system_load_monitoring",
        ]),
    ];

    for (component, tests) in benchmarks {
        println!("\n{}:", component);
        for test in tests {
            println!("  - {}", test);
        }
    }

    Ok(())
}

async fn validate_config_command(cli: &Cli) -> Result<()> {
    info!("Validating configuration file: {}", cli.config);

    match Config::load_from_file(&cli.config) {
        Ok(config) => {
            config.validate()?;
            info!("Configuration file is valid");
        }
        Err(e) => {
            warn!("Configuration file is invalid: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn generate_config_command() -> Result<()> {
    let config = Config::default();
    let config_path = "benchmark_config.toml";
    
    config.save_to_file(config_path)?;
    info!("Generated sample configuration: {}", config_path);

    Ok(())
}

async fn output_results(
    results: &[chronicle_benchmarks::BenchmarkResult],
    format: &str,
    output_file: Option<&str>,
) -> Result<()> {
    let output = match format {
        "json" => serde_json::to_string_pretty(results)?,
        "csv" => results_to_csv(results)?,
        "prometheus" => results_to_prometheus(results)?,
        _ => return Err(anyhow::anyhow!("Unsupported output format: {}", format)),
    };

    if let Some(file_path) = output_file {
        std::fs::write(file_path, output)?;
        info!("Results written to: {}", file_path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn results_to_csv(results: &[chronicle_benchmarks::BenchmarkResult]) -> Result<String> {
    let mut csv = String::new();
    csv.push_str("component,test_name,passed,events_per_second,bytes_per_second,latency_p50_ms,latency_p95_ms,latency_p99_ms,cpu_usage_percent,memory_usage_mb,error_rate\n");

    for result in results {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            result.component,
            result.test_name,
            result.passed,
            result.metrics.throughput.events_per_second,
            result.metrics.throughput.bytes_per_second,
            result.metrics.latency.p50_ms,
            result.metrics.latency.p95_ms,
            result.metrics.latency.p99_ms,
            result.metrics.resources.cpu_usage_percent,
            result.metrics.resources.memory_usage_mb,
            result.metrics.errors.error_rate,
        ));
    }

    Ok(csv)
}

fn results_to_prometheus(results: &[chronicle_benchmarks::BenchmarkResult]) -> Result<String> {
    let mut prometheus = String::new();

    for result in results {
        let labels = format!(
            "{{component=\"{}\",test=\"{}\"}}",
            result.component, result.test_name
        );

        prometheus.push_str(&format!(
            "benchmark_events_per_second{} {}\n",
            labels, result.metrics.throughput.events_per_second
        ));
        prometheus.push_str(&format!(
            "benchmark_bytes_per_second{} {}\n",
            labels, result.metrics.throughput.bytes_per_second
        ));
        prometheus.push_str(&format!(
            "benchmark_latency_p95_milliseconds{} {}\n",
            labels, result.metrics.latency.p95_ms
        ));
        prometheus.push_str(&format!(
            "benchmark_cpu_usage_percent{} {}\n",
            labels, result.metrics.resources.cpu_usage_percent
        ));
        prometheus.push_str(&format!(
            "benchmark_passed{} {}\n",
            labels, if result.passed { 1.0 } else { 0.0 }
        ));
    }

    Ok(prometheus)
}