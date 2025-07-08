import SwiftUI

@main
struct ChronicleUIApp: App {
    @StateObject private var appState = AppState()
    
    var body: some Scene {
        MenuBarExtra("Chronicle", systemImage: "clock.arrow.circlepath") {
            MenuBarView()
                .environmentObject(appState)
        }
        .menuBarExtraStyle(.window)
    }
}