import Foundation
import Combine

@MainActor
class AppState: ObservableObject {
    @Published var isConnected = false
    @Published var systemStatus = SystemStatus()
    @Published var permissionStatus = PermissionStatus()
    @Published var collectors = [CollectorStatus]()
    @Published var backupStatus = BackupStatus()
    @Published var ringBufferStats = RingBufferStats()
    @Published var lastUpdate = Date()
    @Published var errorMessage: String?
    @Published var isLoading = false
    
    private let apiClient = APIClient()
    private let notificationManager = NotificationManager()
    private var updateTimer: Timer?
    
    init() {
        startPeriodicUpdates()
        Task {
            await refreshAllData()
        }
    }
    
    deinit {
        updateTimer?.invalidate()
    }
    
    func startPeriodicUpdates() {
        updateTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            Task { @MainActor in
                await self.refreshAllData()
            }
        }
    }
    
    func stopPeriodicUpdates() {
        updateTimer?.invalidate()
        updateTimer = nil
    }
    
    func refreshAllData() async {
        isLoading = true
        errorMessage = nil
        
        do {
            async let systemStatusTask = apiClient.getSystemStatus()
            async let permissionStatusTask = apiClient.getPermissionStatus()
            async let collectorsTask = apiClient.getCollectors()
            async let backupStatusTask = apiClient.getBackupStatus()
            async let ringBufferStatsTask = apiClient.getRingBufferStats()
            
            systemStatus = try await systemStatusTask
            permissionStatus = try await permissionStatusTask
            collectors = try await collectorsTask
            backupStatus = try await backupStatusTask
            ringBufferStats = try await ringBufferStatsTask
            
            isConnected = true
            lastUpdate = Date()
        } catch {
            isConnected = false
            errorMessage = error.localizedDescription
            await notificationManager.showError("Failed to connect to Chronicle services")
        }
        
        isLoading = false
    }
    
    func toggleCollector(_ collectorId: String) async {
        guard let index = collectors.firstIndex(where: { $0.id == collectorId }) else { return }
        
        let collector = collectors[index]
        let newStatus = !collector.isEnabled
        
        // Optimistically update UI
        collectors[index].isEnabled = newStatus
        
        do {
            try await apiClient.toggleCollector(collectorId, enabled: newStatus)
            await notificationManager.showInfo("Collector \(collector.name) \(newStatus ? "enabled" : "disabled")")
        } catch {
            // Revert on error
            collectors[index].isEnabled = !newStatus
            errorMessage = error.localizedDescription
            await notificationManager.showError("Failed to toggle collector: \(error.localizedDescription)")
        }
    }
    
    func startBackup() async {
        do {
            try await apiClient.startBackup()
            await refreshAllData()
            await notificationManager.showInfo("Backup started")
        } catch {
            errorMessage = error.localizedDescription
            await notificationManager.showError("Failed to start backup: \(error.localizedDescription)")
        }
    }
    
    func search(query: String) async throws -> [SearchResult] {
        return try await apiClient.search(query: query)
    }
}

// MARK: - Data Models

struct SystemStatus: Codable {
    let cpuUsage: Double
    let memoryUsage: Double
    let diskUsage: Double
    let uptime: TimeInterval
    let version: String
    
    init() {
        self.cpuUsage = 0.0
        self.memoryUsage = 0.0
        self.diskUsage = 0.0
        self.uptime = 0.0
        self.version = "Unknown"
    }
}

struct CollectorStatus: Codable, Identifiable {
    let id: String
    let name: String
    var isEnabled: Bool
    let eventCount: Int
    let lastActivity: Date?
    let status: CollectorHealthStatus
    
    enum CollectorHealthStatus: String, Codable, CaseIterable {
        case healthy = "healthy"
        case warning = "warning"
        case error = "error"
        case disabled = "disabled"
    }
}

struct BackupStatus: Codable {
    let isRunning: Bool
    let lastBackup: Date?
    let nextScheduledBackup: Date?
    let totalBackups: Int
    let lastBackupSize: Int64
    let averageBackupTime: TimeInterval
    
    init() {
        self.isRunning = false
        self.lastBackup = nil
        self.nextScheduledBackup = nil
        self.totalBackups = 0
        self.lastBackupSize = 0
        self.averageBackupTime = 0
    }
}

struct RingBufferStats: Codable {
    let totalCapacity: Int
    let currentUsage: Int
    let eventsPerSecond: Double
    let oldestEvent: Date?
    let newestEvent: Date?
    
    init() {
        self.totalCapacity = 0
        self.currentUsage = 0
        self.eventsPerSecond = 0.0
        self.oldestEvent = nil
        self.newestEvent = nil
    }
    
    var usagePercentage: Double {
        guard totalCapacity > 0 else { return 0.0 }
        return Double(currentUsage) / Double(totalCapacity) * 100.0
    }
}

struct SearchResult: Codable, Identifiable {
    let id: String
    let timestamp: Date
    let eventType: String
    let summary: String
    let details: [String: String]
    let relevanceScore: Double
}