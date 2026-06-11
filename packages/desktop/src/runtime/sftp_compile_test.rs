#[cfg(test)]
mod tests {
    #[test]
    fn sftp_types_importable() {
        use russh_sftp::client::SftpSession;
        let _: Option<SftpSession> = None;
    }
}
