import SwiftUI

struct StatusIndicatorComponent: View {
    let status: StatusIndicator.Status
    let size: CGFloat
    let showText: Bool
    
    init(status: StatusIndicator.Status, size: CGFloat = 16, showText: Bool = false) {
        self.status = status
        self.size = size
        self.showText = showText
    }
    
    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: status.systemImage)
                .font(.system(size: size))
                .foregroundColor(status.color)
            
            if showText {
                Text(status.description)
                    .font(.caption)
                    .foregroundColor(status.color)
                    .fontWeight(.medium)
            }
        }
    }
}

// MARK: - Animated Status Indicator

struct AnimatedStatusIndicator: View {
    let status: StatusIndicator.Status
    let size: CGFloat
    let isAnimating: Bool
    
    init(status: StatusIndicator.Status, size: CGFloat = 16, isAnimating: Bool = false) {
        self.status = status
        self.size = size
        self.isAnimating = isAnimating
    }
    
    var body: some View {
        Image(systemName: status.systemImage)
            .font(.system(size: size))
            .foregroundColor(status.color)
            .scaleEffect(isAnimating ? 1.2 : 1.0)
            .animation(.easeInOut(duration: 0.6).repeatForever(autoreverses: true), value: isAnimating)
    }
}

// MARK: - Status Badge

struct StatusBadge: View {
    let status: StatusIndicator.Status
    let text: String
    
    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: status.systemImage)
                .font(.caption2)
                .foregroundColor(status.color)
            
            Text(text)
                .font(.caption2)
                .fontWeight(.medium)
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(
            RoundedRectangle(cornerRadius: 4)
                .fill(status.color.opacity(0.1))
        )
        .overlay(
            RoundedRectangle(cornerRadius: 4)
                .stroke(status.color.opacity(0.3), lineWidth: 1)
        )
    }
}

// MARK: - Preview

struct StatusIndicatorComponent_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 20) {
            HStack(spacing: 20) {
                StatusIndicatorComponent(status: .good, size: 16)
                StatusIndicatorComponent(status: .warning, size: 16)
                StatusIndicatorComponent(status: .error, size: 16)
                StatusIndicatorComponent(status: .unknown, size: 16)
            }
            
            HStack(spacing: 20) {
                StatusIndicatorComponent(status: .good, size: 20, showText: true)
                StatusIndicatorComponent(status: .warning, size: 20, showText: true)
            }
            
            HStack(spacing: 20) {
                AnimatedStatusIndicator(status: .warning, size: 20, isAnimating: true)
                AnimatedStatusIndicator(status: .error, size: 20, isAnimating: true)
            }
            
            HStack(spacing: 8) {
                StatusBadge(status: .good, text: "Online")
                StatusBadge(status: .warning, text: "Limited")
                StatusBadge(status: .error, text: "Offline")
            }
        }
        .padding()
    }
}