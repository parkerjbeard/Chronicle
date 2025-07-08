// Platform abstraction layer for future cross-platform support
import Foundation

protocol PlatformCollectorProtocol {
    associatedtype EventType
    
    func startCapture() async throws
    func stopCapture() async throws
    func supportsFeature(_ feature: CollectorFeature) -> Bool
}

enum CollectorFeature {
    case keyboardCapture
    case screenCapture
    case windowMonitoring
    case mouseTracking
    case clipboardMonitoring
    case fileSystemEvents
    case audioMetadata
    case networkMonitoring
}

// Platform-specific implementations
#if os(macOS)
typealias PlatformKeyCollector = MacOSKeyCollector
typealias PlatformScreenCollector = MacOSScreenCollector
// ... other collectors
#elseif os(Windows)
typealias PlatformKeyCollector = WindowsKeyCollector
typealias PlatformScreenCollector = WindowsScreenCollector
// ... other collectors
#elseif os(Linux)
typealias PlatformKeyCollector = LinuxKeyCollector
typealias PlatformScreenCollector = LinuxScreenCollector
// ... other collectors
#endif

// Generic collector wrapper
struct UniversalCollector<T: PlatformCollectorProtocol> {
    private let platformCollector: T
    
    init() throws {
        #if os(macOS)
        self.platformCollector = T()
        #else
        throw CollectorError.platformNotSupported
        #endif
    }
    
    func start() async throws {
        try await platformCollector.startCapture()
    }
    
    func stop() async throws {
        try await platformCollector.stopCapture()
    }
}

// Future platform implementations (stubs for now)
#if os(Windows)
struct WindowsKeyCollector: PlatformCollectorProtocol {
    // Windows-specific implementation using Windows APIs
}
#endif

#if os(Linux)
struct LinuxKeyCollector: PlatformCollectorProtocol {
    // Linux-specific implementation using X11/Wayland
}
#endif