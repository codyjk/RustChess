//! UCI response formatting for stdout

/// Format UCI responses to send to stdout
pub struct UciResponseFormatter;

impl UciResponseFormatter {
    /// Format the 'uci' command response
    pub fn format_uci_response() -> String {
        "id name RustChess\n\
         id author CJK\n\
         uciok"
            .to_string()
    }

    /// Format the 'isready' command response
    pub fn format_ready_response() -> String {
        "readyok".to_string()
    }

    /// Format the 'bestmove' response
    pub fn format_bestmove_response(best_move: &str) -> String {
        format!("bestmove {}", best_move)
    }

    /// Format search info message
    pub fn format_info(
        depth: u8,
        nodes: usize,
        time_ms: u64,
        score_cp: Option<i16>,
        pv: Option<&str>,
    ) -> String {
        let mut info = format!("info depth {} nodes {} time {}", depth, nodes, time_ms);

        if let Some(cp) = score_cp {
            info.push_str(&format!(" score cp {}", cp));
        }

        if let Some(principal_variation) = pv {
            info.push_str(&format!(" pv {}", principal_variation));
        }

        info
    }

    /// Format error message (not standard UCI, but useful for debugging)
    pub fn format_error(message: &str) -> String {
        format!("info string Error: {}", message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uci_response() {
        let response = UciResponseFormatter::format_uci_response();
        assert!(response.contains("id name RustChess"));
        assert!(response.contains("id author"));
        assert!(response.contains("uciok"));
    }

    #[test]
    fn test_format_ready_response() {
        assert_eq!(UciResponseFormatter::format_ready_response(), "readyok");
    }

    #[test]
    fn test_format_bestmove_response() {
        assert_eq!(
            UciResponseFormatter::format_bestmove_response("e2e4"),
            "bestmove e2e4"
        );
    }

    #[test]
    fn test_format_info() {
        let info = UciResponseFormatter::format_info(6, 123456, 1523, Some(32), Some("e2e4 e7e5"));
        assert!(info.contains("depth 6"));
        assert!(info.contains("nodes 123456"));
        assert!(info.contains("time 1523"));
        assert!(info.contains("score cp 32"));
        assert!(info.contains("pv e2e4 e7e5"));
    }

    #[test]
    fn test_format_info_without_score_and_pv() {
        let info = UciResponseFormatter::format_info(4, 1000, 500, None, None);
        assert_eq!(info, "info depth 4 nodes 1000 time 500");
    }
}
