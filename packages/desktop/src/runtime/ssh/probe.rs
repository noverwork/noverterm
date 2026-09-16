use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Msg};
use russh::ChannelMsg;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::info;
use uuid::Uuid;

use crate::trust::{SshTrustStore, TrustCheck};

use super::authentication::{authenticate_session, client_config};
use super::{
    AuthMethod, ClientHandler, HostSystemInfo, SshProbeHostInfoResponse, SshSessionManager,
};

const SSH_PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const SSH_PROBE_OUTPUT_LIMIT: usize = 16 * 1024;

const HOST_INFO_PROBE_COMMAND: &str = r#"sh -lc '
hostname_value=$(hostname 2>/dev/null || uname -n 2>/dev/null || true)
os_value=$(uname -s 2>/dev/null || true)
cpu_usage_percent_value=""
memory_total_bytes_value=""
memory_used_bytes_value=""
memory_usage_percent_value=""
disk_read_bytes_per_second_value=""
disk_write_bytes_per_second_value=""
network_rx_bytes_per_second_value=""
network_tx_bytes_per_second_value=""
if [ -r /proc/stat ] && [ -r /proc/meminfo ]; then
  cpu_line_1=$(awk "/^cpu / {print}" /proc/stat)
  disk_sample_1=""
  disk_sample_2=""
  network_sample_1=""
  network_sample_2=""
  if [ -r /proc/diskstats ]; then
    disk_sample_1=$(awk "{
      name=\$3;
      if (name ~ /^(loop|ram|zram)/) next;
      if (name ~ /^(sd[a-z]+[0-9]+|vd[a-z]+[0-9]+|xvd[a-z]+[0-9]+|nvme[0-9]+n[0-9]+p[0-9]+|mmcblk[0-9]+p[0-9]+)$/) next;
      read += \$6; write += \$10; found=1;
    }
    END { if (found) printf \"%.0f %.0f\", read * 512, write * 512 }" /proc/diskstats)
  fi
  if [ -r /proc/net/dev ]; then
    network_sample_1=$(awk -F"[: ]+" "NR > 2 {
      iface=\$2;
      if (iface != \"\" && iface != \"lo\") { rx += \$3; tx += \$11; found=1; }
    }
    END { if (found) printf \"%.0f %.0f\", rx, tx }" /proc/net/dev)
  fi
  sleep 1
  cpu_line_2=$(awk "/^cpu / {print}" /proc/stat)
  if [ -r /proc/diskstats ]; then
    disk_sample_2=$(awk "{
      name=\$3;
      if (name ~ /^(loop|ram|zram)/) next;
      if (name ~ /^(sd[a-z]+[0-9]+|vd[a-z]+[0-9]+|xvd[a-z]+[0-9]+|nvme[0-9]+n[0-9]+p[0-9]+|mmcblk[0-9]+p[0-9]+)$/) next;
      read += \$6; write += \$10; found=1;
    }
    END { if (found) printf \"%.0f %.0f\", read * 512, write * 512 }" /proc/diskstats)
  fi
  if [ -r /proc/net/dev ]; then
    network_sample_2=$(awk -F"[: ]+" "NR > 2 {
      iface=\$2;
      if (iface != \"\" && iface != \"lo\") { rx += \$3; tx += \$11; found=1; }
    }
    END { if (found) printf \"%.0f %.0f\", rx, tx }" /proc/net/dev)
  fi
  cpu_usage_percent_value=$(awk -v first="$cpu_line_1" -v second="$cpu_line_2" '\''
    BEGIN {
      split(first, a, " "); n = split(second, b, " ");
      idle1=a[5]+a[6]; idle2=b[5]+b[6];
      total1=0; total2=0;
      for (i=2; i<=n; i++) { total1 += a[i]; total2 += b[i]; }
      total_delta=total2-total1; idle_delta=idle2-idle1;
      if (total_delta > 0) printf "%.1f", (100 * (total_delta - idle_delta) / total_delta);
    }
  '\'')
  memory_total_kb=$(awk "/MemTotal/ {print \$2; exit}" /proc/meminfo)
  memory_available_kb=$(awk "/MemAvailable/ {print \$2; exit}" /proc/meminfo)
  memory_total_bytes_value=$(awk -v value="$memory_total_kb" "BEGIN {if (value > 0) printf \"%.0f\", value * 1024}")
  memory_used_bytes_value=$(awk -v total="$memory_total_kb" -v available="$memory_available_kb" "BEGIN {if (total > 0 && available >= 0) printf \"%.0f\", (total - available) * 1024}")
  memory_usage_percent_value=$(awk -v total="$memory_total_kb" -v available="$memory_available_kb" "BEGIN {if (total > 0 && available >= 0) printf \"%.1f\", 100 * (total - available) / total}")
  disk_values=$(awk -v first="$disk_sample_1" -v second="$disk_sample_2" "BEGIN {
    split(first, a, \" \"  ); split(second, b, \" \"  );
    if (a[1] != \"\" && b[1] != \"\" && b[1] >= a[1]) printf \"%.0f\", b[1] - a[1];
    printf \" \";
    if (a[2] != \"\" && b[2] != \"\" && b[2] >= a[2]) printf \"%.0f\", b[2] - a[2];
  }")
  disk_read_bytes_per_second_value=${disk_values%% *}
  disk_write_bytes_per_second_value=${disk_values#* }
  if [ "$disk_read_bytes_per_second_value" = "$disk_values" ]; then disk_write_bytes_per_second_value=""; fi
  network_values=$(awk -v first="$network_sample_1" -v second="$network_sample_2" "BEGIN {
    split(first, a, \" \"  ); split(second, b, \" \"  );
    if (a[1] != \"\" && b[1] != \"\" && b[1] >= a[1]) printf \"%.0f\", b[1] - a[1];
    printf \" \";
    if (a[2] != \"\" && b[2] != \"\" && b[2] >= a[2]) printf \"%.0f\", b[2] - a[2];
  }")
  network_rx_bytes_per_second_value=${network_values%% *}
  network_tx_bytes_per_second_value=${network_values#* }
  if [ "$network_rx_bytes_per_second_value" = "$network_values" ]; then network_tx_bytes_per_second_value=""; fi
elif command -v vm_stat >/dev/null 2>&1; then
  cpu_usage_percent_value=$(top -l 1 -n 0 2>/dev/null | awk -F"," "/CPU usage/ {gsub(/[^0-9.]/, \"\", \$1); print \$1; exit}")
  memory_total_bytes_value=$(sysctl -n hw.memsize 2>/dev/null || true)
  page_size=$(pagesize 2>/dev/null || echo 4096)
  pages_active=$(vm_stat 2>/dev/null | awk "/Pages active/ {gsub(/[^0-9]/, \"\", \$3); print \$3}")
  pages_wired=$(vm_stat 2>/dev/null | awk "/Pages wired down/ {gsub(/[^0-9]/, \"\", \$4); print \$4}")
  pages_compressed=$(vm_stat 2>/dev/null | awk "/Pages occupied by compressor/ {gsub(/[^0-9]/, \"\", \$5); print \$5}")
  memory_used_bytes_value=$(awk -v active="$pages_active" -v wired="$pages_wired" -v compressed="$pages_compressed" -v size="$page_size" "BEGIN {printf \"%.0f\", (active + wired + compressed) * size}")
  memory_usage_percent_value=$(awk -v used="$memory_used_bytes_value" -v total="$memory_total_bytes_value" "BEGIN {if (total > 0) printf \"%.1f\", 100 * used / total}")
  if command -v netstat >/dev/null 2>&1; then
    network_sample_1=$(netstat -ibn 2>/dev/null | awk "
      NR == 1 {
        for (i=1; i<=NF; i++) {
          if (\$i == \"Ibytes\") rx_col=i;
          if (\$i == \"Obytes\") tx_col=i;
        }
        next;
      }
      rx_col > 0 && tx_col > 0 && \$1 !~ /^lo/ {
        if (\$rx_col ~ /^[0-9]+$/ && \$tx_col ~ /^[0-9]+$/) {
          rx += \$rx_col; tx += \$tx_col; found=1;
        }
      }
      END { if (found) printf \"%.0f %.0f\", rx, tx }")
    sleep 1
    network_sample_2=$(netstat -ibn 2>/dev/null | awk "
      NR == 1 {
        for (i=1; i<=NF; i++) {
          if (\$i == \"Ibytes\") rx_col=i;
          if (\$i == \"Obytes\") tx_col=i;
        }
        next;
      }
      rx_col > 0 && tx_col > 0 && \$1 !~ /^lo/ {
        if (\$rx_col ~ /^[0-9]+$/ && \$tx_col ~ /^[0-9]+$/) {
          rx += \$rx_col; tx += \$tx_col; found=1;
        }
      }
      END { if (found) printf \"%.0f %.0f\", rx, tx }")
    network_values=$(awk -v first="$network_sample_1" -v second="$network_sample_2" "BEGIN {
      split(first, a, \" \"  ); split(second, b, \" \"  );
      if (a[1] != \"\" && b[1] != \"\" && b[1] >= a[1]) printf \"%.0f\", b[1] - a[1];
      printf \" \";
      if (a[2] != \"\" && b[2] != \"\" && b[2] >= a[2]) printf \"%.0f\", b[2] - a[2];
    }")
    network_rx_bytes_per_second_value=${network_values%% *}
    network_tx_bytes_per_second_value=${network_values#* }
    if [ "$network_rx_bytes_per_second_value" = "$network_values" ]; then network_tx_bytes_per_second_value=""; fi
  fi
fi
printf "hostname=%s\n" "$hostname_value"
printf "os=%s\n" "$os_value"
printf "cpu_usage_percent=%s\n" "$cpu_usage_percent_value"
printf "memory_total_bytes=%s\n" "$memory_total_bytes_value"
printf "memory_used_bytes=%s\n" "$memory_used_bytes_value"
printf "memory_usage_percent=%s\n" "$memory_usage_percent_value"
printf "disk_read_bytes_per_second=%s\n" "$disk_read_bytes_per_second_value"
printf "disk_write_bytes_per_second=%s\n" "$disk_write_bytes_per_second_value"
printf "network_rx_bytes_per_second=%s\n" "$network_rx_bytes_per_second_value"
printf "network_tx_bytes_per_second=%s\n" "$network_tx_bytes_per_second_value"
'"#;

impl SshSessionManager {
    pub async fn probe_host_info(
        &self,
        trust_store: SshTrustStore,
        host: String,
        port: u16,
        user: String,
        auth: AuthMethod,
    ) -> Result<SshProbeHostInfoResponse, String> {
        let probe_id = Uuid::new_v4().to_string();
        info!(probe_id, host, port, user, "Starting SSH host info probe");

        let config = client_config(Some(SSH_PROBE_TIMEOUT));
        let trust_check = Arc::new(Mutex::new(None));
        let handler = ClientHandler::new(host.clone(), port, trust_store, trust_check.clone());

        let connect_result = timeout(
            SSH_PROBE_TIMEOUT,
            client::connect(config, (host.clone(), port), handler),
        )
        .await
        .map_err(|_| "SSH probe timed out while connecting".to_string())?;

        let mut session = match connect_result {
            Ok(session) => session,
            Err(error) => return map_probe_connect_error(error, trust_check).await,
        };

        authenticate_session(&mut session, &user, auth, &probe_id).await?;

        let mut channel = timeout(SSH_PROBE_TIMEOUT, session.channel_open_session())
            .await
            .map_err(|_| "SSH probe timed out while opening channel".to_string())?
            .map_err(|error| format!("Failed to open probe channel: {error}"))?;

        timeout(
            SSH_PROBE_TIMEOUT,
            channel.exec(true, HOST_INFO_PROBE_COMMAND.to_string()),
        )
        .await
        .map_err(|_| "SSH probe timed out while starting command".to_string())?
        .map_err(|error| format!("Failed to start host info probe: {error}"))?;

        let output = collect_probe_output(&mut channel).await?;
        let _ = channel.close().await;

        Ok(SshProbeHostInfoResponse::Success {
            info: parse_host_system_info(&output),
        })
    }
}

async fn collect_probe_output(channel: &mut russh::Channel<Msg>) -> Result<String, String> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_status: Option<u32> = None;

    let collect = async {
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => {
                    if stdout.len() + data.len() > SSH_PROBE_OUTPUT_LIMIT {
                        return Err("SSH probe output exceeded limit".to_string());
                    }
                    stdout.extend_from_slice(&data);
                }
                ChannelMsg::ExtendedData { data, .. } => {
                    if stderr.len() + data.len() > SSH_PROBE_OUTPUT_LIMIT {
                        return Err("SSH probe error output exceeded limit".to_string());
                    }
                    stderr.extend_from_slice(&data);
                }
                ChannelMsg::ExitStatus {
                    exit_status: status,
                } => {
                    exit_status = Some(status);
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        Ok::<(), String>(())
    };

    timeout(SSH_PROBE_TIMEOUT, collect)
        .await
        .map_err(|_| "SSH probe timed out while reading output".to_string())??;

    if !matches!(exit_status, None | Some(0)) {
        let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(format!(
                "SSH probe exited with status {}",
                exit_status.unwrap_or(1)
            ));
        }
        return Err(format!(
            "SSH probe exited with status {}: {stderr}",
            exit_status.unwrap_or(1)
        ));
    }

    Ok(String::from_utf8_lossy(&stdout).to_string())
}

fn parse_host_system_info(output: &str) -> HostSystemInfo {
    let value = |key: &str| {
        output.lines().find_map(|line| {
            let (candidate_key, candidate_value) = line.split_once('=')?;
            if candidate_key == key {
                let value = candidate_value.trim();
                if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            } else {
                None
            }
        })
    };

    HostSystemInfo {
        hostname: value("hostname"),
        os: value("os"),
        cpu_usage_percent: value("cpu_usage_percent").and_then(|value| value.parse::<f64>().ok()),
        memory_total_bytes: value("memory_total_bytes").and_then(|value| value.parse::<f64>().ok()),
        memory_used_bytes: value("memory_used_bytes").and_then(|value| value.parse::<f64>().ok()),
        memory_usage_percent: value("memory_usage_percent")
            .and_then(|value| value.parse::<f64>().ok()),
        disk_read_bytes_per_second: value("disk_read_bytes_per_second")
            .and_then(|value| value.parse::<f64>().ok()),
        disk_write_bytes_per_second: value("disk_write_bytes_per_second")
            .and_then(|value| value.parse::<f64>().ok()),
        network_rx_bytes_per_second: value("network_rx_bytes_per_second")
            .and_then(|value| value.parse::<f64>().ok()),
        network_tx_bytes_per_second: value("network_tx_bytes_per_second")
            .and_then(|value| value.parse::<f64>().ok()),
    }
}

async fn map_probe_connect_error(
    error: russh::Error,
    trust_check: Arc<Mutex<Option<TrustCheck>>>,
) -> Result<SshProbeHostInfoResponse, String> {
    match trust_check.lock().await.clone() {
        Some(TrustCheck::TrustRequired(prompt)) => {
            Ok(SshProbeHostInfoResponse::TrustRequired { prompt })
        }
        Some(TrustCheck::TrustMismatch(mismatch)) => {
            Ok(SshProbeHostInfoResponse::TrustMismatch { mismatch })
        }
        Some(TrustCheck::Trusted) | None => Err(format!("Failed to connect: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_host_system_info;

    #[test]
    fn parse_host_system_info_reads_io_metrics() {
        let info = parse_host_system_info(
            "hostname=buildbox\nos=Linux\ncpu_usage_percent=17.4\nmemory_total_bytes=17179869184\nmemory_used_bytes=8589934592\nmemory_usage_percent=50.0\ndisk_read_bytes_per_second=4096\ndisk_write_bytes_per_second=8192\nnetwork_rx_bytes_per_second=123456\nnetwork_tx_bytes_per_second=654321\n",
        );

        assert_eq!(info.hostname, Some("buildbox".to_string()));
        assert_eq!(info.os, Some("Linux".to_string()));
        assert_eq!(info.cpu_usage_percent, Some(17.4));
        assert_eq!(info.memory_total_bytes, Some(17_179_869_184.0));
        assert_eq!(info.memory_used_bytes, Some(8_589_934_592.0));
        assert_eq!(info.memory_usage_percent, Some(50.0));
        assert_eq!(info.disk_read_bytes_per_second, Some(4096.0));
        assert_eq!(info.disk_write_bytes_per_second, Some(8192.0));
        assert_eq!(info.network_rx_bytes_per_second, Some(123_456.0));
        assert_eq!(info.network_tx_bytes_per_second, Some(654_321.0));
    }

    #[test]
    fn parse_host_system_info_ignores_missing_or_invalid_io_metrics() {
        let info = parse_host_system_info(
            "hostname=\nos=Darwin\ncpu_usage_percent=bad\nmemory_total_bytes=\ndisk_read_bytes_per_second=not-a-number\ndisk_write_bytes_per_second=\nnetwork_rx_bytes_per_second=42\n",
        );

        assert_eq!(info.hostname, None);
        assert_eq!(info.os, Some("Darwin".to_string()));
        assert_eq!(info.cpu_usage_percent, None);
        assert_eq!(info.memory_total_bytes, None);
        assert_eq!(info.disk_read_bytes_per_second, None);
        assert_eq!(info.disk_write_bytes_per_second, None);
        assert_eq!(info.network_rx_bytes_per_second, Some(42.0));
        assert_eq!(info.network_tx_bytes_per_second, None);
    }
}
