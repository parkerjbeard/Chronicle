//
//  BasicUsage.swift
//  ChronicleCollectors Examples
//
//  Created by Chronicle on 2024-01-01.
//  Copyright © 2024 Chronicle. All rights reserved.
//

import Foundation
import ChronicleCollectors

/// Example demonstrating basic usage of Chronicle Collectors
class BasicUsageExample {
    
    private let collectors = ChronicleCollectors.shared
    
    func run() async {
        print("Chronicle Collectors Basic Usage Example")
        print("=========================================")
        
        // 1. Check available collectors
        print("\n1. Available Collectors:")
        let availableTypes = collectors.getAvailableCollectorTypes()
        for (id, name) in availableTypes {
            print("  - \(id): \(name)")
        }
        
        // 2. Check permissions
        print("\n2. Checking Permissions:")
        let permissions = collectors.checkAllPermissions()
        for (type, status) in permissions {
            print("  - \(type.displayName): \(status)")
        }
        
        // 3. Request permissions if needed
        if !collectors.hasAllRequiredPermissions() {
            print("\n3. Requesting Permissions...")
            do {
                try await collectors.requestAllPermissions()
                print("  ✓ Permissions granted")
            } catch {
                print("  ✗ Failed to get permissions: \(error)")
                return
            }
        } else {
            print("\n3. All required permissions already granted ✓")
        }
        
        // 4. Configure collectors
        print("\n4. Configuring Collectors...")
        do {
            // Enable keyboard monitoring with reduced sample rate
            try collectors.updateCollectorConfiguration("key_tap", config: CollectorConfiguration(
                enabled: true,
                sampleRate: 0.5, // 50% sampling
                adaptiveFrameRate: true,
                activeFrameRate: 5.0,
                idleFrameRate: 1.0
            ))
            
            // Enable window monitoring
            try collectors.updateCollectorConfiguration("window_mon", config: CollectorConfiguration(
                enabled: true,
                sampleRate: 1.0,
                activeFrameRate: 2.0,
                idleFrameRate: 0.5
            ))
            
            print("  ✓ Collectors configured")
        } catch {
            print("  ✗ Failed to configure collectors: \(error)")
            return
        }
        
        // 5. Start collectors
        print("\n5. Starting Collectors...")
        do {
            try await collectors.startCollectors()
            print("  ✓ Collectors started")
            print("  Running collectors: \(collectors.runningCollectors.joined(separator: ", "))")
        } catch {
            print("  ✗ Failed to start collectors: \(error)")
            return
        }
        
        // 6. Monitor for a while
        print("\n6. Monitoring (will run for 30 seconds)...")
        print("  Please interact with your computer to generate events...")
        
        for i in 1...6 {
            try await Task.sleep(nanoseconds: 5_000_000_000) // 5 seconds
            
            let health = collectors.getSystemHealth()
            let ringBufferStats = collectors.getRingBufferStatistics()
            
            print("  \(i*5)s - Events: \(health.totalEventsCollected), Errors: \(health.totalErrors), Buffer: \(String(format: "%.1f", ringBufferStats.utilizationPercentage))%")
        }
        
        // 7. Get detailed statistics
        print("\n7. Final Statistics:")
        let allStats = collectors.getAllStatistics()
        for (id, stats) in allStats {
            if stats.eventsCollected > 0 {
                print("  \(id):")
                print("    Events: \(stats.eventsCollected)")
                print("    Dropped: \(stats.eventsDropped)")
                print("    Errors: \(stats.errorCount)")
                print("    Avg Size: \(String(format: "%.1f", stats.averageEventSize)) bytes")
                print("    Frame Rate: \(String(format: "%.1f", stats.currentFrameRate)) fps")
            }
        }
        
        // 8. Stop collectors
        print("\n8. Stopping Collectors...")
        do {
            try collectors.stopCollectors()
            print("  ✓ Collectors stopped")
        } catch {
            print("  ✗ Failed to stop collectors: \(error)")
        }
        
        print("\nExample completed! ✓")
    }
}

/// Example demonstrating individual collector usage
class IndividualCollectorExample {
    
    private let collectors = ChronicleCollectors.shared
    
    func run() async {
        print("Individual Collector Example")
        print("============================")
        
        // Start only specific collectors
        print("\n1. Starting only keyboard and window monitoring...")
        
        do {
            try collectors.startCollector("key_tap")
            try collectors.startCollector("window_mon")
            
            print("  ✓ Started key_tap and window_mon")
            
            // Monitor for 10 seconds
            try await Task.sleep(nanoseconds: 10_000_000_000)
            
            // Get statistics for specific collectors
            print("\n2. Statistics:")
            if let keyStats = collectors.getCollectorStatistics("key_tap") {
                print("  Key Tap - Events: \(keyStats.eventsCollected), Uptime: \(String(format: "%.1f", keyStats.uptime))s")
            }
            
            if let windowStats = collectors.getCollectorStatistics("window_mon") {
                print("  Window Mon - Events: \(windowStats.eventsCollected), Uptime: \(String(format: "%.1f", windowStats.uptime))s")
            }
            
            // Stop individual collectors
            try collectors.stopCollector("key_tap")
            try collectors.stopCollector("window_mon")
            
            print("  ✓ Stopped collectors")
            
        } catch {
            print("  ✗ Error: \(error)")
        }
    }
}

/// Example demonstrating configuration management
class ConfigurationExample {
    
    func run() {
        print("Configuration Management Example")
        print("================================")
        
        let configManager = ConfigManager.shared
        
        // 1. Show current configuration
        print("\n1. Current Configuration:")
        let config = configManager.config
        print("  App Name: \(config.general.appName)")
        print("  Version: \(config.general.appVersion)")
        print("  Ring Buffer Size: \(config.ringBuffer.bufferSize) bytes")
        print("  Data Retention: \(config.privacy.dataRetentionDays) days")
        
        // 2. Show collector configurations
        print("\n2. Collector Configurations:")
        for (id, collectorConfig) in config.collectors {
            print("  \(id):")
            print("    Enabled: \(collectorConfig.enabled)")
            print("    Sample Rate: \(collectorConfig.sampleRate)")
            print("    Active FPS: \(collectorConfig.activeFrameRate)")
            print("    Idle FPS: \(collectorConfig.idleFrameRate)")
        }
        
        // 3. Validate configuration
        print("\n3. Validating Configuration:")
        do {
            try config.validate()
            print("  ✓ Configuration is valid")
        } catch {
            print("  ✗ Configuration validation failed: \(error)")
        }
    }
}

// MARK: - Main Entry Point

@main
struct ChronicleCollectorsExample {
    static func main() async {
        print("Chronicle Collectors Framework Examples")
        print("======================================")
        
        // Run basic usage example
        await BasicUsageExample().run()
        
        print("\n" + String(repeating: "=", count: 50) + "\n")
        
        // Run individual collector example
        await IndividualCollectorExample().run()
        
        print("\n" + String(repeating: "=", count: 50) + "\n")
        
        // Run configuration example
        ConfigurationExample().run()
    }
}