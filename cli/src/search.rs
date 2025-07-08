use crate::api::SearchQuery;
use crate::error::{ChronicleError, Result};
use chrono::{DateTime, Utc, NaiveDateTime, TimeZone, Duration};
use regex::Regex;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self> {
        if start > end {
            return Err(ChronicleError::InvalidTimeRange(
                "Start time must be before end time".to_string(),
            ));
        }
        Ok(Self { start, end })
    }

    pub fn from_relative(relative: &str) -> Result<Self> {
        let now = Utc::now();
        let (start, end) = match relative.to_lowercase().as_str() {
            "today" => {
                let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let end = now.date_naive().and_hms_opt(23, 59, 59).unwrap();
                (Utc.from_utc_datetime(&start), Utc.from_utc_datetime(&end))
            }
            "yesterday" => {
                let yesterday = now - Duration::days(1);
                let start = yesterday.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let end = yesterday.date_naive().and_hms_opt(23, 59, 59).unwrap();
                (Utc.from_utc_datetime(&start), Utc.from_utc_datetime(&end))
            }
            "last-hour" => {
                let start = now - Duration::hours(1);
                (start, now)
            }
            "last-day" => {
                let start = now - Duration::days(1);
                (start, now)
            }
            "last-week" => {
                let start = now - Duration::weeks(1);
                (start, now)
            }
            "last-month" => {
                let start = now - Duration::days(30);
                (start, now)
            }
            "last-year" => {
                let start = now - Duration::days(365);
                (start, now)
            }
            _ => {
                return Err(ChronicleError::InvalidTimeRange(format!(
                    "Unknown relative time: {}. Supported: today, yesterday, last-hour, last-day, last-week, last-month, last-year",
                    relative
                )));
            }
        };
        
        Self::new(start, end)
    }

    pub fn parse(time_str: &str) -> Result<Self> {
        // Handle relative time strings
        if !time_str.contains("..") && !time_str.contains("to") {
            return Self::from_relative(time_str);
        }

        // Handle range formats
        let parts: Vec<&str> = if time_str.contains("..") {
            time_str.split("..").collect()
        } else if time_str.contains(" to ") {
            time_str.split(" to ").collect()
        } else {
            return Err(ChronicleError::InvalidTimeRange(
                "Time range must be in format 'start..end' or 'start to end'".to_string(),
            ));
        };

        if parts.len() != 2 {
            return Err(ChronicleError::InvalidTimeRange(
                "Time range must have exactly two parts".to_string(),
            ));
        }

        let start = Self::parse_datetime(parts[0].trim())?;
        let end = Self::parse_datetime(parts[1].trim())?;

        Self::new(start, end)
    }

    fn parse_datetime(datetime_str: &str) -> Result<DateTime<Utc>> {
        // Try various datetime formats
        let formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M",
            "%Y-%m-%d",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%Y-%m-%dT%H:%M",
            "%m/%d/%Y %H:%M:%S",
            "%m/%d/%Y %H:%M",
            "%m/%d/%Y",
            "%d/%m/%Y %H:%M:%S",
            "%d/%m/%Y %H:%M",
            "%d/%m/%Y",
        ];

        for format in &formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(datetime_str, format) {
                return Ok(Utc.from_utc_datetime(&naive_dt));
            }
        }

        // Try parsing as RFC3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        Err(ChronicleError::InvalidTimeRange(format!(
            "Unable to parse datetime: {}. Supported formats: YYYY-MM-DD, YYYY-MM-DD HH:MM:SS, ISO8601, etc.",
            datetime_str
        )))
    }
}

#[derive(Debug, Clone)]
pub struct SearchQueryBuilder {
    query: String,
    time_range: Option<TimeRange>,
    limit: Option<usize>,
    offset: Option<usize>,
    filters: HashMap<String, String>,
}

impl SearchQueryBuilder {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            time_range: None,
            limit: None,
            offset: None,
            filters: HashMap::new(),
        }
    }

    pub fn with_time_range(mut self, time_range: TimeRange) -> Self {
        self.time_range = Some(time_range);
        self
    }

    pub fn with_time_str(mut self, time_str: &str) -> Result<Self> {
        self.time_range = Some(TimeRange::parse(time_str)?);
        Ok(self)
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_filter(mut self, key: &str, value: &str) -> Self {
        self.filters.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> SearchQuery {
        SearchQuery {
            query: self.query,
            start_time: self.time_range.as_ref().map(|tr| tr.start),
            end_time: self.time_range.as_ref().map(|tr| tr.end),
            limit: self.limit,
            offset: self.offset,
            filters: if self.filters.is_empty() {
                None
            } else {
                Some(self.filters)
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryValidator {
    regex: Option<Regex>,
}

impl QueryValidator {
    pub fn new() -> Self {
        Self { regex: None }
    }

    pub fn validate_query(&mut self, query: &str) -> Result<()> {
        if query.is_empty() {
            return Err(ChronicleError::InvalidQuery("Query cannot be empty".to_string()));
        }

        // Try to compile as regex to validate syntax
        if query.starts_with('/') && query.ends_with('/') {
            let regex_pattern = &query[1..query.len() - 1];
            match Regex::new(regex_pattern) {
                Ok(re) => {
                    self.regex = Some(re);
                    Ok(())
                }
                Err(e) => Err(ChronicleError::InvalidQuery(format!(
                    "Invalid regex pattern: {}",
                    e
                ))),
            }
        } else {
            // For non-regex queries, basic validation
            if query.len() > 1000 {
                return Err(ChronicleError::InvalidQuery(
                    "Query too long (max 1000 characters)".to_string(),
                ));
            }
            Ok(())
        }
    }

    pub fn is_regex(&self) -> bool {
        self.regex.is_some()
    }
}

pub fn parse_filters(filter_str: &str) -> Result<HashMap<String, String>> {
    let mut filters = HashMap::new();
    
    for pair in filter_str.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(ChronicleError::InvalidQuery(format!(
                "Invalid filter format: '{}'. Expected key=value",
                pair
            )));
        }
        
        let key = parts[0].trim();
        let value = parts[1].trim();
        
        if key.is_empty() || value.is_empty() {
            return Err(ChronicleError::InvalidQuery(format!(
                "Filter key and value cannot be empty: '{}'",
                pair
            )));
        }
        
        filters.insert(key.to_string(), value.to_string());
    }
    
    Ok(filters)
}

pub fn validate_limit(limit: usize) -> Result<()> {
    if limit == 0 {
        return Err(ChronicleError::InvalidQuery("Limit must be greater than 0".to_string()));
    }
    if limit > 10000 {
        return Err(ChronicleError::InvalidQuery("Limit cannot exceed 10000".to_string()));
    }
    Ok(())
}

pub fn validate_offset(offset: usize) -> Result<()> {
    if offset > 1000000 {
        return Err(ChronicleError::InvalidQuery("Offset cannot exceed 1,000,000".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_parsing() {
        // Test relative times
        assert!(TimeRange::from_relative("today").is_ok());
        assert!(TimeRange::from_relative("yesterday").is_ok());
        assert!(TimeRange::from_relative("last-week").is_ok());
        assert!(TimeRange::from_relative("invalid").is_err());

        // Test range parsing
        assert!(TimeRange::parse("2024-01-01..2024-01-02").is_ok());
        assert!(TimeRange::parse("2024-01-01 to 2024-01-02").is_ok());
        assert!(TimeRange::parse("today").is_ok());
        assert!(TimeRange::parse("invalid").is_err());
    }

    #[test]
    fn test_query_validation() {
        let mut validator = QueryValidator::new();
        
        assert!(validator.validate_query("test").is_ok());
        assert!(validator.validate_query("/test.*/").is_ok());
        assert!(validator.validate_query("").is_err());
        assert!(validator.validate_query("/[/").is_err());
    }

    #[test]
    fn test_filter_parsing() {
        let filters = parse_filters("type=error,level=critical").unwrap();
        assert_eq!(filters.get("type"), Some(&"error".to_string()));
        assert_eq!(filters.get("level"), Some(&"critical".to_string()));
        
        assert!(parse_filters("invalid").is_err());
        assert!(parse_filters("=value").is_err());
        assert!(parse_filters("key=").is_err());
    }

    #[test]
    fn test_search_query_builder() {
        let query = SearchQueryBuilder::new("test")
            .with_limit(100)
            .with_offset(10)
            .with_filter("type", "error")
            .build();
        
        assert_eq!(query.query, "test");
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(10));
        assert!(query.filters.is_some());
    }
}