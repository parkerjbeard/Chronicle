import SwiftUI

struct MenuBarView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedTab: MenuTab = .status
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            headerView
            
            Divider()
            
            // Tab Navigation
            tabNavigationView
            
            Divider()
            
            // Content
            contentView
                .frame(width: 350, height: 400)
            
            Divider()
            
            // Footer
            footerView
        }
        .background(Color(NSColor.windowBackgroundColor))
        .cornerRadius(8)
        .shadow(color: .black.opacity(0.1), radius: 8, x: 0, y: 4)
    }
    
    private var headerView: some View {
        HStack {
            Image(systemName: "clock.arrow.circlepath")
                .font(.title2)
                .foregroundColor(.accentColor)
            
            VStack(alignment: .leading) {
                Text("Chronicle")
                    .font(.headline)
                    .fontWeight(.semibold)
                
                Text(appState.isConnected ? "Connected" : "Disconnected")
                    .font(.caption)
                    .foregroundColor(appState.isConnected ? .green : .red)
            }
            
            Spacer()
            
            connectionIndicator
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }
    
    private var connectionIndicator: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(appState.isConnected ? Color.green : Color.red)
                .frame(width: 8, height: 8)
            
            if appState.isLoading {
                ProgressView()
                    .scaleEffect(0.5)
                    .frame(width: 12, height: 12)
            }
        }
    }
    
    private var tabNavigationView: some View {
        HStack(spacing: 0) {
            ForEach(MenuTab.allCases, id: \.self) { tab in
                Button(action: {
                    selectedTab = tab
                }) {
                    VStack(spacing: 2) {
                        Image(systemName: tab.systemImage)
                            .font(.system(size: 16))
                        Text(tab.title)
                            .font(.caption2)
                    }
                    .foregroundColor(selectedTab == tab ? .accentColor : .secondary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                    .background(
                        selectedTab == tab ? Color.accentColor.opacity(0.1) : Color.clear
                    )
                }
                .buttonStyle(PlainButtonStyle())
            }
        }
        .padding(.horizontal, 4)
    }
    
    private var contentView: some View {
        Group {
            switch selectedTab {
            case .status:
                StatusView()
            case .permissions:
                PermissionView()
            case .backup:
                BackupView()
            case .search:
                SearchView()
            case .settings:
                SettingsView()
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }
    
    private var footerView: some View {
        HStack {
            Text("Last updated: \(formattedLastUpdate)")
                .font(.caption2)
                .foregroundColor(.secondary)
            
            Spacer()
            
            Button("Refresh") {
                Task {
                    await appState.refreshAllData()
                }
            }
            .font(.caption)
            .buttonStyle(PlainButtonStyle())
            .foregroundColor(.accentColor)
            
            Button("Quit") {
                NSApplication.shared.terminate(nil)
            }
            .font(.caption)
            .buttonStyle(PlainButtonStyle())
            .foregroundColor(.red)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
    }
    
    private var formattedLastUpdate: String {
        let formatter = DateFormatter()
        formatter.timeStyle = .medium
        return formatter.string(from: appState.lastUpdate)
    }
}

// MARK: - Menu Tab Definition

enum MenuTab: String, CaseIterable {
    case status = "Status"
    case permissions = "Permissions"
    case backup = "Backup"
    case search = "Search"
    case settings = "Settings"
    
    var title: String {
        return rawValue
    }
    
    var systemImage: String {
        switch self {
        case .status:
            return "chart.bar.fill"
        case .permissions:
            return "lock.shield"
        case .backup:
            return "externaldrive"
        case .search:
            return "magnifyingglass"
        case .settings:
            return "gear"
        }
    }
}

// MARK: - Preview

struct MenuBarView_Previews: PreviewProvider {
    static var previews: some View {
        MenuBarView()
            .environmentObject(AppState())
    }
}