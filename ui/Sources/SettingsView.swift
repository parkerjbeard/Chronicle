import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedTab: SettingsTab = .general
    
    var body: some View {
        VStack(spacing: 0) {
            // Settings Tab Navigation
            settingsTabNavigation
            
            Divider()
            
            // Settings Content
            settingsContent
        }
    }
    
    private var settingsTabNavigation: some View {
        HStack(spacing: 0) {
            ForEach(SettingsTab.allCases, id: \.self) { tab in
                Button(action: {
                    selectedTab = tab
                }) {
                    VStack(spacing: 2) {
                        Image(systemName: tab.systemImage)
                            .font(.caption)
                        Text(tab.title)
                            .font(.caption2)
                    }
                    .foregroundColor(selectedTab == tab ? .accentColor : .secondary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
                    .background(
                        selectedTab == tab ? Color.accentColor.opacity(0.1) : Color.clear
                    )
                }
                .buttonStyle(PlainButtonStyle())
            }
        }
        .padding(.horizontal, 4)
        .padding(.vertical, 4)
    }
    
    private var settingsContent: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                switch selectedTab {
                case .general:
                    GeneralPreferences()
                case .collectors:
                    CollectorPreferences()
                case .advanced:
                    AdvancedPreferences()
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
        }
    }
}

// MARK: - Settings Tab Definition

enum SettingsTab: String, CaseIterable {
    case general = "General"
    case collectors = "Collectors"
    case advanced = "Advanced"
    
    var title: String {
        return rawValue
    }
    
    var systemImage: String {
        switch self {
        case .general:
            return "gear"
        case .collectors:
            return "sensor.tag.radiowaves.forward"
        case .advanced:
            return "wrench.and.screwdriver"
        }
    }
}

// MARK: - Preview

struct SettingsView_Previews: PreviewProvider {
    static var previews: some View {
        SettingsView()
            .environmentObject(AppState())
            .frame(width: 350, height: 400)
    }
}