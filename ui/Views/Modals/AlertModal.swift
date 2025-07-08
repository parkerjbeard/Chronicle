import SwiftUI

struct AlertModal: View {
    let title: String
    let message: String
    let alertType: AlertType
    let primaryAction: AlertAction?
    let secondaryAction: AlertAction?
    
    @Environment(\.dismiss) private var dismiss
    
    init(
        title: String,
        message: String,
        type: AlertType = .info,
        primaryAction: AlertAction? = nil,
        secondaryAction: AlertAction? = nil
    ) {
        self.title = title
        self.message = message
        self.alertType = type
        self.primaryAction = primaryAction
        self.secondaryAction = secondaryAction
    }
    
    var body: some View {
        VStack(spacing: 20) {
            // Icon
            Image(systemName: alertType.systemImage)
                .font(.system(size: 48))
                .foregroundColor(alertType.color)
            
            // Title
            Text(title)
                .font(.headline)
                .fontWeight(.semibold)
                .multilineTextAlignment(.center)
            
            // Message
            Text(message)
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            
            // Actions
            HStack(spacing: 12) {
                if let secondaryAction = secondaryAction {
                    Button(secondaryAction.title) {
                        secondaryAction.handler?()
                        dismiss()
                    }
                    .buttonStyle(SecondaryButtonStyle())
                }
                
                if let primaryAction = primaryAction {
                    Button(primaryAction.title) {
                        primaryAction.handler?()
                        dismiss()
                    }
                    .buttonStyle(PrimaryButtonStyle(color: alertType.color))
                } else {
                    Button("OK") {
                        dismiss()
                    }
                    .buttonStyle(PrimaryButtonStyle(color: alertType.color))
                }
            }
        }
        .padding(24)
        .background(Color(NSColor.windowBackgroundColor))
        .cornerRadius(12)
        .shadow(color: .black.opacity(0.1), radius: 20, x: 0, y: 10)
        .frame(maxWidth: 400)
    }
}

// MARK: - Alert Type

enum AlertType {
    case info
    case warning
    case error
    case success
    
    var systemImage: String {
        switch self {
        case .info:
            return "info.circle"
        case .warning:
            return "exclamationmark.triangle"
        case .error:
            return "xmark.circle"
        case .success:
            return "checkmark.circle"
        }
    }
    
    var color: Color {
        switch self {
        case .info:
            return .blue
        case .warning:
            return .orange
        case .error:
            return .red
        case .success:
            return .green
        }
    }
}

// MARK: - Alert Action

struct AlertAction {
    let title: String
    let handler: (() -> Void)?
    
    init(title: String, handler: (() -> Void)? = nil) {
        self.title = title
        self.handler = handler
    }
}

// MARK: - Button Styles

struct PrimaryButtonStyle: ButtonStyle {
    let color: Color
    
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.body)
            .fontWeight(.medium)
            .foregroundColor(.white)
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(color)
            .cornerRadius(8)
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}

struct SecondaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.body)
            .fontWeight(.medium)
            .foregroundColor(.primary)
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(Color.gray.opacity(0.1))
            .cornerRadius(8)
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}

// MARK: - Convenience Extensions

extension View {
    func alertModal(
        isPresented: Binding<Bool>,
        title: String,
        message: String,
        type: AlertType = .info,
        primaryAction: AlertAction? = nil,
        secondaryAction: AlertAction? = nil
    ) -> some View {
        self.sheet(isPresented: isPresented) {
            AlertModal(
                title: title,
                message: message,
                type: type,
                primaryAction: primaryAction,
                secondaryAction: secondaryAction
            )
        }
    }
}

// MARK: - Preview

struct AlertModal_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 20) {
            AlertModal(
                title: "Information",
                message: "This is an informational alert message.",
                type: .info
            )
            
            AlertModal(
                title: "Warning",
                message: "This is a warning alert message that requires attention.",
                type: .warning,
                primaryAction: AlertAction(title: "Continue"),
                secondaryAction: AlertAction(title: "Cancel")
            )
            
            AlertModal(
                title: "Error",
                message: "An error has occurred. Please try again.",
                type: .error,
                primaryAction: AlertAction(title: "Retry"),
                secondaryAction: AlertAction(title: "Cancel")
            )
        }
        .padding()
        .background(Color.black.opacity(0.3))
    }
}