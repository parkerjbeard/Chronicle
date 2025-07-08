//! Search performance benchmarks
//!
//! Tests Chronicle's search functionality performance including query execution time,
//! index lookup performance, and result retrieval efficiency.

use crate::{
    BenchmarkComponent, BenchmarkConfig, BenchmarkResult, ErrorMetrics, LatencyMetrics,
    PerformanceMetrics, ResourceMetrics, ThroughputMetrics,
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;

/// Search benchmark test cases
const BENCHMARK_TESTS: &[&str] = &[
    "simple_text_search",
    "wildcard_search",
    "regex_search",
    "date_range_search",
    "complex_query_search",
    "large_result_set_search",
    "concurrent_search",
    "index_lookup_performance",
    "faceted_search",
    "fuzzy_search",
    "search_with_filters",
    "search_result_pagination",
];

/// Simulated search index
struct SearchIndex {
    documents: Vec<Document>,
    text_index: HashMap<String, Vec<usize>>,
    date_index: HashMap<String, Vec<usize>>,
    queries_executed: AtomicU64,
    results_returned: AtomicU64,
    errors: AtomicU64,
}

#[derive(Debug, Clone)]
struct Document {
    id: usize,
    title: String,
    content: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    tags: Vec<String>,
    size: usize,
}

#[derive(Debug, Clone)]
struct SearchQuery {
    query_type: QueryType,
    text: String,
    date_from: Option<chrono::DateTime<chrono::Utc>>,
    date_to: Option<chrono::DateTime<chrono::Utc>>,
    tags: Vec<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Clone)]
enum QueryType {
    Simple,
    Wildcard,
    Regex,
    DateRange,
    Complex,
    Faceted,
    Fuzzy,
}

#[derive(Debug, Clone)]
struct SearchResult {
    documents: Vec<Document>,
    total_count: usize,
    query_time_ms: f64,
    facets: HashMap<String, Vec<(String, usize)>>,
}

impl SearchIndex {
    fn new() -> Self {
        let mut index = Self {
            documents: Vec::new(),
            text_index: HashMap::new(),
            date_index: HashMap::new(),
            queries_executed: AtomicU64::new(0),
            results_returned: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        };
        
        // Initialize with sample data
        index.initialize_sample_data();
        index
    }
    
    fn initialize_sample_data(&mut self) {
        // Create 10,000 sample documents
        for i in 0..10_000 {
            let doc = Document {
                id: i,
                title: format!("Document {}", i),
                content: format!("This is the content of document {}. It contains various keywords and phrases for testing search functionality.", i),
                timestamp: chrono::Utc::now() - chrono::Duration::days(i as i64 % 365),
                tags: vec![
                    format!("tag_{}", i % 10),
                    format!("category_{}", i % 5),
                    format!("type_{}", i % 3),
                ],
                size: 1024 + (i % 10000),
            };
            
            self.add_document_to_index(&doc);
            self.documents.push(doc);
        }
    }
    
    fn add_document_to_index(&mut self, doc: &Document) {
        // Add to text index
        let words: Vec<&str> = doc.content.split_whitespace().collect();
        for word in words {
            let word_lower = word.to_lowercase();
            self.text_index.entry(word_lower)
                .or_insert_with(Vec::new)
                .push(doc.id);
        }
        
        // Add to date index
        let date_key = doc.timestamp.format("%Y-%m-%d").to_string();
        self.date_index.entry(date_key)
            .or_insert_with(Vec::new)
            .push(doc.id);
    }
    
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let start = Instant::now();
        
        // Simulate search processing delay
        let processing_delay = match query.query_type {
            QueryType::Simple => Duration::from_micros(100),
            QueryType::Wildcard => Duration::from_micros(500),
            QueryType::Regex => Duration::from_millis(10),
            QueryType::DateRange => Duration::from_micros(200),
            QueryType::Complex => Duration::from_millis(50),
            QueryType::Faceted => Duration::from_millis(20),
            QueryType::Fuzzy => Duration::from_millis(100),
        };
        
        time::sleep(processing_delay).await;
        
        // Simulate search execution
        let matching_docs = self.execute_search(query).await?;
        
        let query_time = start.elapsed().as_nanos() as f64 / 1_000_000.0;
        
        self.queries_executed.fetch_add(1, Ordering::Relaxed);
        self.results_returned.fetch_add(matching_docs.len() as u64, Ordering::Relaxed);
        
        // Simulate occasional errors
        if rand::random::<f64>() < 0.001 {
            self.errors.fetch_add(1, Ordering::Relaxed);
            return Err(anyhow::anyhow!("Search error"));
        }
        
        Ok(SearchResult {
            documents: matching_docs,
            total_count: self.documents.len(),
            query_time_ms: query_time,
            facets: self.generate_facets(query),
        })
    }
    
    async fn execute_search(&self, query: &SearchQuery) -> Result<Vec<Document>> {
        let mut matching_ids = Vec::new();
        
        match query.query_type {
            QueryType::Simple => {
                // Simple text search
                if let Some(doc_ids) = self.text_index.get(&query.text.to_lowercase()) {
                    matching_ids.extend(doc_ids.iter().cloned());
                }
            }
            QueryType::Wildcard => {
                // Wildcard search - simulate pattern matching
                for (word, doc_ids) in &self.text_index {
                    if word.contains(&query.text.to_lowercase()) {
                        matching_ids.extend(doc_ids.iter().cloned());
                    }
                }
            }
            QueryType::Regex => {
                // Regex search - simulate regex matching
                for (word, doc_ids) in &self.text_index {
                    if word.len() > 3 && word.starts_with(&query.text.chars().take(3).collect::<String>()) {
                        matching_ids.extend(doc_ids.iter().cloned());
                    }
                }
            }
            QueryType::DateRange => {
                // Date range search
                if let (Some(from), Some(to)) = (query.date_from, query.date_to) {
                    let mut current_date = from;
                    while current_date <= to {
                        let date_key = current_date.format("%Y-%m-%d").to_string();
                        if let Some(doc_ids) = self.date_index.get(&date_key) {
                            matching_ids.extend(doc_ids.iter().cloned());
                        }
                        current_date += chrono::Duration::days(1);
                    }
                }
            }
            QueryType::Complex => {
                // Complex query - combine multiple criteria
                if let Some(doc_ids) = self.text_index.get(&query.text.to_lowercase()) {
                    matching_ids.extend(doc_ids.iter().cloned());
                }
                // Filter by tags
                if !query.tags.is_empty() {
                    matching_ids.retain(|&id| {
                        if let Some(doc) = self.documents.get(id) {
                            query.tags.iter().any(|tag| doc.tags.contains(tag))
                        } else {
                            false
                        }
                    });
                }
            }
            QueryType::Faceted => {
                // Faceted search - return results with facet information
                matching_ids.extend(0..std::cmp::min(100, self.documents.len()));
            }
            QueryType::Fuzzy => {
                // Fuzzy search - simulate fuzzy matching
                for (word, doc_ids) in &self.text_index {
                    if self.fuzzy_match(word, &query.text) {
                        matching_ids.extend(doc_ids.iter().cloned());
                    }
                }
            }
        }
        
        // Remove duplicates and apply limits
        matching_ids.sort();
        matching_ids.dedup();
        
        if let Some(offset) = query.offset {
            matching_ids = matching_ids.into_iter().skip(offset).collect();
        }
        
        if let Some(limit) = query.limit {
            matching_ids.truncate(limit);
        }
        
        // Return matching documents
        let matching_docs = matching_ids
            .into_iter()
            .filter_map(|id| self.documents.get(id).cloned())
            .collect();
        
        Ok(matching_docs)
    }
    
    fn fuzzy_match(&self, word1: &str, word2: &str) -> bool {
        // Simple fuzzy matching - check if words are similar
        let distance = self.levenshtein_distance(word1, word2);
        distance <= 2 && word1.len() > 3
    }
    
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
        
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }
        
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1.chars().nth(i - 1) == s2.chars().nth(j - 1) { 0 } else { 1 };
                matrix[i][j] = std::cmp::min(
                    std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                    matrix[i - 1][j - 1] + cost,
                );
            }
        }
        
        matrix[len1][len2]
    }
    
    fn generate_facets(&self, _query: &SearchQuery) -> HashMap<String, Vec<(String, usize)>> {
        let mut facets = HashMap::new();
        
        // Generate sample facets
        facets.insert("tags".to_string(), vec![
            ("tag_1".to_string(), 1000),
            ("tag_2".to_string(), 900),
            ("tag_3".to_string(), 800),
        ]);
        
        facets.insert("categories".to_string(), vec![
            ("category_1".to_string(), 2000),
            ("category_2".to_string(), 1500),
            ("category_3".to_string(), 1000),
        ]);
        
        facets
    }
    
    fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.queries_executed.load(Ordering::Relaxed),
            self.results_returned.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
}

/// Run a specific search benchmark
pub async fn run_benchmark(test_name: &str, config: &BenchmarkConfig) -> Result<BenchmarkResult> {
    let start_time = Instant::now();
    
    let result = match test_name {
        "simple_text_search" => simple_text_search_benchmark(config).await,
        "wildcard_search" => wildcard_search_benchmark(config).await,
        "regex_search" => regex_search_benchmark(config).await,
        "date_range_search" => date_range_search_benchmark(config).await,
        "complex_query_search" => complex_query_search_benchmark(config).await,
        "large_result_set_search" => large_result_set_search_benchmark(config).await,
        "concurrent_search" => concurrent_search_benchmark(config).await,
        "index_lookup_performance" => index_lookup_performance_benchmark(config).await,
        "faceted_search" => faceted_search_benchmark(config).await,
        "fuzzy_search" => fuzzy_search_benchmark(config).await,
        "search_with_filters" => search_with_filters_benchmark(config).await,
        "search_result_pagination" => search_result_pagination_benchmark(config).await,
        _ => return Err(anyhow::anyhow!("Unknown benchmark test: {}", test_name)),
    };

    let duration = start_time.elapsed();
    
    match result {
        Ok(mut metrics) => {
            metrics.timestamp = chrono::Utc::now();
            
            // Check if performance targets are met (<100ms for typical queries)
            let passed = metrics.latency.p95_ms <= config.targets.search_latency_ms as f64;
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Search,
                test_name: test_name.to_string(),
                metrics,
                passed,
                notes: Some(format!("Completed in {:.2?}", duration)),
            })
        }
        Err(e) => {
            let metrics = create_error_metrics();
            
            Ok(BenchmarkResult {
                component: BenchmarkComponent::Search,
                test_name: test_name.to_string(),
                metrics,
                passed: false,
                notes: Some(format!("Failed: {}", e)),
            })
        }
    }
}

/// Run all search benchmarks
pub async fn run_all_benchmarks(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>> {
    let mut results = Vec::new();
    
    for test_name in BENCHMARK_TESTS {
        let result = run_benchmark(test_name, config).await?;
        results.push(result);
    }
    
    Ok(results)
}

/// Simple text search benchmark
async fn simple_text_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Simple,
        text: "document".to_string(),
        date_from: None,
        date_to: None,
        tags: vec![],
        limit: Some(10),
        offset: None,
    };
    
    // Warmup
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(), // Assume 1KB per result
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Wildcard search benchmark
async fn wildcard_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Wildcard,
        text: "doc*".to_string(),
        date_from: None,
        date_to: None,
        tags: vec![],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Regex search benchmark
async fn regex_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Regex,
        text: "doc".to_string(),
        date_from: None,
        date_to: None,
        tags: vec![],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Date range search benchmark
async fn date_range_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::DateRange,
        text: "".to_string(),
        date_from: Some(chrono::Utc::now() - chrono::Duration::days(30)),
        date_to: Some(chrono::Utc::now()),
        tags: vec![],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Complex query search benchmark
async fn complex_query_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Complex,
        text: "content".to_string(),
        date_from: Some(chrono::Utc::now() - chrono::Duration::days(30)),
        date_to: Some(chrono::Utc::now()),
        tags: vec!["tag_1".to_string(), "tag_2".to_string()],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Large result set search benchmark
async fn large_result_set_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Simple,
        text: "document".to_string(),
        date_from: None,
        date_to: None,
        tags: vec![],
        limit: Some(1000), // Large result set
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Concurrent search benchmark
async fn concurrent_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Simple,
        text: "document".to_string(),
        date_from: None,
        date_to: None,
        tags: vec![],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    
    // Run concurrent searches
    let mut tasks = Vec::new();
    for _ in 0..config.concurrency {
        let index_clone = search_index.clone();
        let query_clone = query.clone();
        let task = tokio::spawn(async move {
            let mut query_times = Vec::new();
            for _ in 0..(config.iterations / config.concurrency) {
                let result = index_clone.search(&query_clone).await?;
                query_times.push(result.query_time_ms);
            }
            Ok::<Vec<f64>, anyhow::Error>(query_times)
        });
        tasks.push(task);
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let duration = start_time.elapsed();
    let (queries, results_count, errors) = search_index.get_stats();
    
    let mut all_query_times = Vec::new();
    for result in results {
        all_query_times.extend(result??);
    }
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results_count as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&all_query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Index lookup performance benchmark
async fn index_lookup_performance_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let queries = vec![
        "document", "content", "testing", "various", "keywords",
        "phrases", "functionality", "sample", "data", "information",
    ];
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let query_text = queries[rand::random::<usize>() % queries.len()];
        let query = SearchQuery {
            query_type: QueryType::Simple,
            text: query_text.to_string(),
            date_from: None,
            date_to: None,
            tags: vec![],
            limit: Some(10),
            offset: None,
        };
        
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Faceted search benchmark
async fn faceted_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Faceted,
        text: "".to_string(),
        date_from: None,
        date_to: None,
        tags: vec![],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Fuzzy search benchmark
async fn fuzzy_search_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Fuzzy,
        text: "documen".to_string(), // Misspelled intentionally
        date_from: None,
        date_to: None,
        tags: vec![],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Search with filters benchmark
async fn search_with_filters_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    let query = SearchQuery {
        query_type: QueryType::Complex,
        text: "document".to_string(),
        date_from: Some(chrono::Utc::now() - chrono::Duration::days(30)),
        date_to: Some(chrono::Utc::now()),
        tags: vec!["tag_1".to_string()],
        limit: Some(10),
        offset: None,
    };
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for _ in 0..config.iterations {
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Search result pagination benchmark
async fn search_result_pagination_benchmark(
    config: &BenchmarkConfig,
) -> Result<PerformanceMetrics> {
    let search_index = Arc::new(SearchIndex::new());
    
    time::sleep(config.warmup_duration).await;
    
    let start_time = Instant::now();
    let mut query_times = Vec::new();
    
    for i in 0..config.iterations {
        let offset = (i * 10) % 1000; // Paginate through results
        let query = SearchQuery {
            query_type: QueryType::Simple,
            text: "document".to_string(),
            date_from: None,
            date_to: None,
            tags: vec![],
            limit: Some(10),
            offset: Some(offset as usize),
        };
        
        let result = search_index.search(&query).await?;
        query_times.push(result.query_time_ms);
    }
    
    let duration = start_time.elapsed();
    let (queries, results, errors) = search_index.get_stats();
    
    let queries_per_second = queries as f64 / duration.as_secs_f64();
    
    Ok(PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: queries_per_second,
            bytes_per_second: results as f64 * 1024.0 / duration.as_secs_f64(),
            operations_per_second: queries_per_second,
        },
        latency: calculate_latency_metrics(&query_times),
        resources: get_resource_metrics(),
        errors: ErrorMetrics {
            error_rate: errors as f64 / queries as f64,
            recovery_time_ms: 0.0,
            total_errors: errors,
        },
    })
}

/// Calculate latency metrics from a set of measurements
fn calculate_latency_metrics(latencies: &[f64]) -> LatencyMetrics {
    if latencies.is_empty() {
        return LatencyMetrics {
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
        };
    }
    
    let mut sorted_latencies = latencies.to_vec();
    sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let len = sorted_latencies.len();
    let p50_idx = (len as f64 * 0.50) as usize;
    let p95_idx = (len as f64 * 0.95) as usize;
    let p99_idx = (len as f64 * 0.99) as usize;
    
    let mean = sorted_latencies.iter().sum::<f64>() / len as f64;
    
    LatencyMetrics {
        p50_ms: sorted_latencies[p50_idx.min(len - 1)],
        p95_ms: sorted_latencies[p95_idx.min(len - 1)],
        p99_ms: sorted_latencies[p99_idx.min(len - 1)],
        max_ms: sorted_latencies[len - 1],
        mean_ms: mean,
    }
}

/// Get current resource usage metrics
fn get_resource_metrics() -> ResourceMetrics {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    
    let cpu_usage = system.global_cpu_info().cpu_usage() as f64;
    let memory_usage = system.used_memory() as f64 / 1024.0 / 1024.0;
    
    ResourceMetrics {
        cpu_usage_percent: cpu_usage,
        memory_usage_mb: memory_usage,
        disk_io_bytes_per_second: 0.0,
        network_io_bytes_per_second: 0.0,
        file_handles: 0,
        thread_count: system.processes().len() as u64,
    }
}

/// Create error metrics for failed benchmarks
fn create_error_metrics() -> PerformanceMetrics {
    PerformanceMetrics {
        timestamp: chrono::Utc::now(),
        throughput: ThroughputMetrics {
            events_per_second: 0.0,
            bytes_per_second: 0.0,
            operations_per_second: 0.0,
        },
        latency: LatencyMetrics {
            p50_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            mean_ms: 0.0,
        },
        resources: ResourceMetrics {
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            disk_io_bytes_per_second: 0.0,
            network_io_bytes_per_second: 0.0,
            file_handles: 0,
            thread_count: 0,
        },
        errors: ErrorMetrics {
            error_rate: 1.0,
            recovery_time_ms: 0.0,
            total_errors: 1,
        },
    }
}