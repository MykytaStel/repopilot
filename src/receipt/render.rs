use crate::receipt::model::AuditReceipt;

pub fn render_receipt_json(receipt: &AuditReceipt) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(receipt)
}
