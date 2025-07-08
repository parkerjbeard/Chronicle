import SwiftUI

struct SearchView: View {
    @EnvironmentObject var appState: AppState
    @State private var searchQuery = ""
    @State private var searchResults: [SearchResult] = []
    @State private var isSearching = false
    @State private var selectedResult: SearchResult?
    @State private var showingResultDetail = false
    
    var body: some View {
        VStack(spacing: 0) {
            // Search Input
            searchInputSection
            
            Divider()
            
            // Results
            if isSearching {
                searchingView
            } else if searchResults.isEmpty && !searchQuery.isEmpty {
                noResultsView
            } else if searchResults.isEmpty {
                emptyStateView
            } else {
                searchResultsView
            }
        }
    }
    
    private var searchInputSection: some View {
        VStack(spacing: 8) {
            HStack {
                Image(systemName: "magnifyingglass")
                    .font(.caption)
                    .foregroundColor(.secondary)
                
                TextField("Search Chronicle data...", text: $searchQuery)
                    .textFieldStyle(PlainTextFieldStyle())
                    .font(.caption)
                    .onSubmit {
                        performSearch()
                    }
                
                if !searchQuery.isEmpty {
                    Button(action: {
                        searchQuery = ""
                        searchResults = []
                    }) {
                        Image(systemName: "xmark.circle.fill")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(PlainButtonStyle())
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Color.gray.opacity(0.1))
            .cornerRadius(6)
            
            HStack {
                Text("Search examples:")
                    .font(.caption2)
                    .foregroundColor(.secondary)
                
                Button("app:finder") {
                    searchQuery = "app:finder"
                    performSearch()
                }
                .font(.caption2)
                .foregroundColor(.accentColor)
                .buttonStyle(PlainButtonStyle())
                
                Button("type:file") {
                    searchQuery = "type:file"
                    performSearch()
                }
                .font(.caption2)
                .foregroundColor(.accentColor)
                .buttonStyle(PlainButtonStyle())
                
                Spacer()
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }
    
    private var searchingView: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
            
            Text("Searching...")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    
    private var noResultsView: some View {
        VStack(spacing: 16) {
            Image(systemName: "magnifyingglass")
                .font(.largeTitle)
                .foregroundColor(.secondary)
            
            Text("No results found")
                .font(.caption)
                .fontWeight(.medium)
                .foregroundColor(.secondary)
            
            Text("Try adjusting your search terms or check if the data exists in the specified time range.")
                .font(.caption2)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 20)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    
    private var emptyStateView: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.text.magnifyingglass")
                .font(.largeTitle)
                .foregroundColor(.secondary)
            
            Text("Search Chronicle Data")
                .font(.caption)
                .fontWeight(.medium)
                .foregroundColor(.secondary)
            
            VStack(alignment: .leading, spacing: 4) {
                Text("Search tips:")
                    .font(.caption2)
                    .fontWeight(.medium)
                    .foregroundColor(.secondary)
                
                Text("• Use app:name to find application events")
                Text("• Use type:file for file system events")
                Text("• Use before:date or after:date for time ranges")
                Text("• Combine terms with AND/OR operators")
            }
            .font(.caption2)
            .foregroundColor(.secondary)
            .padding(.horizontal, 20)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    
    private var searchResultsView: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(searchResults) { result in
                    SearchResultRow(result: result) {
                        selectedResult = result
                        showingResultDetail = true
                    }
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
        }
        .sheet(isPresented: $showingResultDetail) {
            if let result = selectedResult {
                SearchResultDetailView(result: result)
            }
        }
    }
    
    // MARK: - Actions
    
    private func performSearch() {
        guard !searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return
        }
        
        isSearching = true
        
        Task {
            do {
                let results = try await appState.search(query: searchQuery)
                await MainActor.run {
                    searchResults = results
                    isSearching = false
                }
            } catch {
                await MainActor.run {
                    searchResults = []
                    isSearching = false
                    // Show error message
                }
            }
        }
    }
}

// MARK: - Supporting Views

struct SearchResultRow: View {
    let result: SearchResult
    let onTap: () -> Void
    
    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Image(systemName: eventTypeIcon(result.eventType))
                        .font(.caption)
                        .foregroundColor(eventTypeColor(result.eventType))
                        .frame(width: 16)
                    
                    Text(result.eventType.capitalized)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                        .fontWeight(.medium)
                    
                    Spacer()
                    
                    Text(formatTimestamp(result.timestamp))
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                Text(result.summary)
                    .font(.caption)
                    .fontWeight(.medium)
                    .multilineTextAlignment(.leading)
                    .lineLimit(2)
                
                if result.relevanceScore > 0.8 {
                    HStack {
                        Image(systemName: "star.fill")
                            .font(.caption2)
                            .foregroundColor(.yellow)
                        
                        Text("High relevance")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .background(Color.gray.opacity(0.05))
            .cornerRadius(6)
        }
        .buttonStyle(PlainButtonStyle())
    }
    
    private func eventTypeIcon(_ type: String) -> String {
        switch type.lowercased() {
        case "file":
            return "doc"
        case "app":
            return "app"
        case "network":
            return "network"
        case "keyboard":
            return "keyboard"
        case "mouse":
            return "cursorarrow.click"
        case "screen":
            return "display"
        case "window":
            return "macwindow"
        case "clipboard":
            return "doc.on.clipboard"
        case "audio":
            return "speaker.wave.2"
        default:
            return "circle"
        }
    }
    
    private func eventTypeColor(_ type: String) -> Color {
        switch type.lowercased() {
        case "file":
            return .blue
        case "app":
            return .purple
        case "network":
            return .green
        case "keyboard":
            return .orange
        case "mouse":
            return .red
        case "screen":
            return .cyan
        case "window":
            return .pink
        case "clipboard":
            return .yellow
        case "audio":
            return .mint
        default:
            return .gray
        }
    }
    
    private func formatTimestamp(_ timestamp: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.dateTimeStyle = .named
        return formatter.localizedString(for: timestamp, relativeTo: Date())
    }
}

struct SearchResultDetailView: View {
    let result: SearchResult
    @Environment(\.dismiss) private var dismiss
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    // Header
                    VStack(alignment: .leading, spacing: 8) {
                        Text(result.eventType.capitalized)
                            .font(.title3)
                            .fontWeight(.bold)
                        
                        Text(result.summary)
                            .font(.body)
                            .foregroundColor(.secondary)
                        
                        Text(formatDetailTimestamp(result.timestamp))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                    Divider()
                    
                    // Details
                    if !result.details.isEmpty {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Details")
                                .font(.headline)
                            
                            ForEach(result.details.sorted(by: { $0.key < $1.key }), id: \.key) { key, value in
                                DetailRow(key: key, value: value)
                            }
                        }
                    }
                    
                    // Relevance
                    if result.relevanceScore > 0 {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Search Relevance")
                                .font(.headline)
                            
                            HStack {
                                Text("Score:")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                
                                Text(String(format: "%.2f", result.relevanceScore))
                                    .font(.caption)
                                    .fontWeight(.medium)
                                
                                Spacer()
                                
                                ProgressView(value: result.relevanceScore)
                                    .progressViewStyle(LinearProgressViewStyle())
                                    .frame(width: 100)
                            }
                        }
                    }
                    
                    Spacer()
                }
                .padding()
            }
            .navigationTitle("Search Result")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
        .frame(width: 500, height: 400)
    }
    
    private func formatDetailTimestamp(_ timestamp: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .medium
        return formatter.string(from: timestamp)
    }
}

struct DetailRow: View {
    let key: String
    let value: String
    
    var body: some View {
        HStack(alignment: .top) {
            Text(key.capitalized + ":")
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(width: 80, alignment: .leading)
            
            Text(value)
                .font(.caption)
                .fontWeight(.medium)
                .multilineTextAlignment(.leading)
            
            Spacer()
        }
    }
}

// MARK: - Preview

struct SearchView_Previews: PreviewProvider {
    static var previews: some View {
        SearchView()
            .environmentObject(AppState())
            .frame(width: 350, height: 400)
    }
}