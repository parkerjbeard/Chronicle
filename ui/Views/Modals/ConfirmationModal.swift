import SwiftUI

struct ConfirmationModal: View {
    let title: String
    let message: String
    let confirmText: String
    let cancelText: String
    let isDestructive: Bool
    let onConfirm: () -> Void
    let onCancel: () -> Void
    
    @Environment(\.dismiss) private var dismiss
    @State private var isProcessing = false
    
    init(
        title: String,
        message: String,
        confirmText: String = "Confirm",
        cancelText: String = "Cancel",
        isDestructive: Bool = false,
        onConfirm: @escaping () -> Void,
        onCancel: @escaping () -> Void = {}
    ) {
        self.title = title
        self.message = message
        self.confirmText = confirmText
        self.cancelText = cancelText
        self.isDestructive = isDestructive
        self.onConfirm = onConfirm
        self.onCancel = onCancel
    }
    
    var body: some View {
        VStack(spacing: 20) {
            // Icon
            Image(systemName: isDestructive ? "exclamationmark.triangle" : "questionmark.circle")
                .font(.system(size: 48))
                .foregroundColor(isDestructive ? .red : .blue)
            
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
                Button(cancelText) {
                    onCancel()
                    dismiss()
                }
                .buttonStyle(SecondaryButtonStyle())
                .disabled(isProcessing)
                
                Button(confirmText) {
                    handleConfirm()
                }
                .buttonStyle(PrimaryButtonStyle(color: isDestructive ? .red : .blue))
                .disabled(isProcessing)
            }
        }
        .padding(24)
        .background(Color(NSColor.windowBackgroundColor))
        .cornerRadius(12)
        .shadow(color: .black.opacity(0.1), radius: 20, x: 0, y: 10)
        .frame(maxWidth: 400)
    }
    
    private func handleConfirm() {
        isProcessing = true
        
        // Add slight delay to show processing state
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            onConfirm()
            dismiss()
        }
    }
}

// MARK: - Specialized Confirmation Modals

struct DeleteConfirmationModal: View {
    let itemName: String
    let onConfirm: () -> Void
    let onCancel: () -> Void
    
    var body: some View {
        ConfirmationModal(
            title: "Delete \(itemName)?",
            message: "This action cannot be undone. Are you sure you want to delete \(itemName)?",
            confirmText: "Delete",
            cancelText: "Cancel",
            isDestructive: true,
            onConfirm: onConfirm,
            onCancel: onCancel
        )
    }
}

struct WipeDataConfirmationModal: View {
    let daysToWipe: Int
    let onConfirm: () -> Void
    let onCancel: () -> Void
    
    var body: some View {
        ConfirmationModal(
            title: "Wipe Data?",
            message: "This will permanently delete all Chronicle data older than \(daysToWipe) days. This action cannot be undone.",
            confirmText: "Wipe Data",
            cancelText: "Cancel",
            isDestructive: true,
            onConfirm: onConfirm,
            onCancel: onCancel
        )
    }
}

struct ResetSettingsConfirmationModal: View {
    let onConfirm: () -> Void
    let onCancel: () -> Void
    
    var body: some View {
        ConfirmationModal(
            title: "Reset Settings?",
            message: "This will reset all Chronicle settings to their default values. You will need to reconfigure your preferences.",
            confirmText: "Reset",
            cancelText: "Cancel",
            isDestructive: true,
            onConfirm: onConfirm,
            onCancel: onCancel
        )
    }
}

struct StopAllCollectorsConfirmationModal: View {
    let onConfirm: () -> Void
    let onCancel: () -> Void
    
    var body: some View {
        ConfirmationModal(
            title: "Stop All Collectors?",
            message: "This will stop all active collectors. No new data will be collected until you restart them.",
            confirmText: "Stop All",
            cancelText: "Cancel",
            isDestructive: false,
            onConfirm: onConfirm,
            onCancel: onCancel
        )
    }
}

// MARK: - Convenience Extensions

extension View {
    func confirmationModal(
        isPresented: Binding<Bool>,
        title: String,
        message: String,
        confirmText: String = "Confirm",
        cancelText: String = "Cancel",
        isDestructive: Bool = false,
        onConfirm: @escaping () -> Void,
        onCancel: @escaping () -> Void = {}
    ) -> some View {
        self.sheet(isPresented: isPresented) {
            ConfirmationModal(
                title: title,
                message: message,
                confirmText: confirmText,
                cancelText: cancelText,
                isDestructive: isDestructive,
                onConfirm: onConfirm,
                onCancel: onCancel
            )
        }
    }
    
    func deleteConfirmationModal(
        isPresented: Binding<Bool>,
        itemName: String,
        onConfirm: @escaping () -> Void,
        onCancel: @escaping () -> Void = {}
    ) -> some View {
        self.sheet(isPresented: isPresented) {
            DeleteConfirmationModal(
                itemName: itemName,
                onConfirm: onConfirm,
                onCancel: onCancel
            )
        }
    }
    
    func wipeDataConfirmationModal(
        isPresented: Binding<Bool>,
        daysToWipe: Int,
        onConfirm: @escaping () -> Void,
        onCancel: @escaping () -> Void = {}
    ) -> some View {
        self.sheet(isPresented: isPresented) {
            WipeDataConfirmationModal(
                daysToWipe: daysToWipe,
                onConfirm: onConfirm,
                onCancel: onCancel
            )
        }
    }
}

// MARK: - Preview

struct ConfirmationModal_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 20) {
            ConfirmationModal(
                title: "Save Changes?",
                message: "You have unsaved changes. Do you want to save them before closing?",
                confirmText: "Save",
                cancelText: "Don't Save",
                isDestructive: false,
                onConfirm: {},
                onCancel: {}
            )
            
            DeleteConfirmationModal(
                itemName: "backup_2024_01_15.db",
                onConfirm: {},
                onCancel: {}
            )
            
            WipeDataConfirmationModal(
                daysToWipe: 30,
                onConfirm: {},
                onCancel: {}
            )
        }
        .padding()
        .background(Color.black.opacity(0.3))
    }
}