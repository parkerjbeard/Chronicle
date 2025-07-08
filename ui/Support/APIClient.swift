import Foundation
import Network

class APIClient: ObservableObject {
    private let baseURL = URL(string: "http://localhost:8080")!
    private let session = URLSession.shared
    private let monitor = NWPathMonitor()
    private let queue = DispatchQueue(label: "APIClient")
    
    @Published var isConnected = false
    
    init() {
        startNetworkMonitoring()
    }
    
    deinit {
        monitor.cancel()
    }
    
    // MARK: - Network Monitoring
    
    private func startNetworkMonitoring() {
        monitor.pathUpdateHandler = { [weak self] path in
            DispatchQueue.main.async {
                self?.isConnected = path.status == .satisfied
            }
        }
        monitor.start(queue: queue)
    }
    
    // MARK: - API Endpoints
    
    func getSystemStatus() async throws -> SystemStatus {
        let url = baseURL.appendingPathComponent("api/status/system")
        return try await performRequest(url: url)
    }
    
    func getPermissionStatus() async throws -> PermissionStatus {
        let url = baseURL.appendingPathComponent("api/status/permissions")
        return try await performRequest(url: url)
    }
    
    func getCollectors() async throws -> [CollectorStatus] {
        let url = baseURL.appendingPathComponent("api/collectors")
        return try await performRequest(url: url)
    }
    
    func toggleCollector(_ collectorId: String, enabled: Bool) async throws {
        let url = baseURL.appendingPathComponent("api/collectors/\(collectorId)/toggle")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body = ["enabled": enabled]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        let (_, response) = try await session.data(for: request)
        try validateResponse(response)
    }
    
    func getBackupStatus() async throws -> BackupStatus {
        let url = baseURL.appendingPathComponent("api/backup/status")
        return try await performRequest(url: url)
    }
    
    func startBackup() async throws {
        let url = baseURL.appendingPathComponent("api/backup/start")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        
        let (_, response) = try await session.data(for: request)
        try validateResponse(response)
    }
    
    func getRingBufferStats() async throws -> RingBufferStats {
        let url = baseURL.appendingPathComponent("api/ring-buffer/stats")
        return try await performRequest(url: url)
    }
    
    func search(query: String) async throws -> [SearchResult] {
        let url = baseURL.appendingPathComponent("api/search")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body = ["query": query]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        let (data, response) = try await session.data(for: request)
        try validateResponse(response)
        
        return try JSONDecoder().decode([SearchResult].self, from: data)
    }
    
    func getConfiguration() async throws -> Configuration {
        let url = baseURL.appendingPathComponent("api/config")
        return try await performRequest(url: url)
    }
    
    func updateConfiguration(_ config: Configuration) async throws {
        let url = baseURL.appendingPathComponent("api/config")
        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        request.httpBody = try encoder.encode(config)
        
        let (_, response) = try await session.data(for: request)
        try validateResponse(response)
    }
    
    func exportData(format: ExportFormat, dateRange: DateInterval) async throws -> Data {
        let url = baseURL.appendingPathComponent("api/export")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body = [
            "format": format.rawValue,
            "start_date": ISO8601DateFormatter().string(from: dateRange.start),
            "end_date": ISO8601DateFormatter().string(from: dateRange.end)
        ]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        let (data, response) = try await session.data(for: request)
        try validateResponse(response)
        
        return data
    }
    
    func wipeDatabaseOlderThan(days: Int) async throws {
        let url = baseURL.appendingPathComponent("api/wipe")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        
        let body = ["days": days]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)
        
        let (_, response) = try await session.data(for: request)
        try validateResponse(response)
    }
    
    // MARK: - Private Methods
    
    private func performRequest<T: Codable>(url: URL) async throws -> T {
        let (data, response) = try await session.data(from: url)
        try validateResponse(response)
        
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return try decoder.decode(T.self, from: data)
    }
    
    private func validateResponse(_ response: URLResponse) throws {
        guard let httpResponse = response as? HTTPURLResponse else {
            throw APIError.invalidResponse
        }
        
        guard 200..<300 ~= httpResponse.statusCode else {
            throw APIError.httpError(httpResponse.statusCode)
        }
    }
}

// MARK: - Supporting Types

enum APIError: LocalizedError {
    case invalidResponse
    case httpError(Int)
    case networkError
    case decodingError
    
    var errorDescription: String? {
        switch self {
        case .invalidResponse:
            return "Invalid response received"
        case .httpError(let code):
            return "HTTP error: \(code)"
        case .networkError:
            return "Network connection error"
        case .decodingError:
            return "Failed to decode response"
        }
    }
}

struct Configuration: Codable {
    let general: GeneralConfig
    let collectors: CollectorConfig
    let advanced: AdvancedConfig
}

struct GeneralConfig: Codable {
    let autoStart: Bool
    let showNotifications: Bool
    let logLevel: String
    let updateInterval: TimeInterval
}

struct CollectorConfig: Codable {
    let keyboard: Bool
    let mouse: Bool
    let screen: Bool
    let audio: Bool
    let files: Bool
    let network: Bool
    let window: Bool
    let clipboard: Bool
}

struct AdvancedConfig: Codable {
    let ringBufferSize: Int
    let compressionLevel: Int
    let encryptionEnabled: Bool
    let backupInterval: TimeInterval
    let retentionDays: Int
}

enum ExportFormat: String, CaseIterable {
    case json = "json"
    case csv = "csv"
    case sqlite = "sqlite"
    
    var displayName: String {
        switch self {
        case .json:
            return "JSON"
        case .csv:
            return "CSV"
        case .sqlite:
            return "SQLite"
        }
    }
    
    var fileExtension: String {
        switch self {
        case .json:
            return "json"
        case .csv:
            return "csv"
        case .sqlite:
            return "db"
        }
    }
}