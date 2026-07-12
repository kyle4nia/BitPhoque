# VPS Node Quick-Check Commands

_All commands run from any local terminal with SSH access to the Hetzner VPS._
_Target: `root@89.167.32.239` (Hetzner CX33, Helsinki, TN10 kaspad)_

---

## 1. Full health snapshot (the everything command)

```
ssh root@89.167.32.239 "echo '=== KASPAD ===' && systemctl status kaspad --no-pager -l | head -15 && echo && echo '=== RECENT SYNC ===' && journalctl -u kaspad --no-pager -n 10 | grep -i 'processed\|IBD\|sync\|tip' && echo && echo '=== CPU/MEM ===' && ps aux --sort=-%mem | head -3 && echo && echo '=== DISK ===' && df -h /dev/sdb && echo && echo '=== SWAP ===' && free -h && echo && echo '=== UPTIME ===' && uptime"
```

**What you're reading:**
- `Active: active (running)` = kaspad is up. Anything else = problem.
- `Processed N blocks` lines = sync throughput. High N (200-300/10s) = catching up. Low N (1-10/10s) = at or near tip.
- `IBD: Processed X blocks (Y%)` = still syncing, Y% done.
- RSS column in `ps aux` = actual RAM used (in KB). Divide by ~1M for GB.
- `/dev/sdb` = the 200G data volume. Watch `Use%` — worry above 85%.

---

## 2. Is the node synced? (IBD check)

```
ssh root@89.167.32.239 "journalctl -u kaspad --no-pager -n 30 | grep -i -E 'IBD|processed|sync rate|tip'"
```

**Reading the output:**
- If you see `IBD: Processed X blocks (Y%)` lines → still syncing, Y% done. The timestamp in the log line tells you how far behind the tip.
- If you see only `Processed 1 blocks` with no IBD prefix → at the tip, processing live blocks.
- `Sync rate recovered` = node had a slow patch but is back to normal.

---

## 3. Is kaspad alive?

```
ssh root@89.167.32.239 "systemctl is-active kaspad && systemctl status kaspad --no-pager | head -10"
```

Returns `active` if running. If dead, restart with:

```
ssh root@89.167.32.239 "sudo systemctl restart kaspad && sleep 3 && systemctl status kaspad --no-pager | head -10"
```

---

## 4. Resource pressure (RAM + swap + CPU load)

```
ssh root@89.167.32.239 "free -h && echo && uptime"
```

**What matters:**
- `Swap used` under 4G = comfortable. Over 6G = getting tight.
- Load average under 4.0 = fine (4 vCPUs). Sustained above 4 = saturated.
- If `Mem available` drops below 500M and swap is climbing = OOM risk.

---

## 5. Disk usage (data volume)

```
ssh root@89.167.32.239 "df -h /dev/sdb"
```

The data volume is 200G (`/mnt/HC_Volume_106172981`). If `Use%` passes 85%, consider whether the node is still syncing (will grow) or at the tip (stable).

---

## 6. Recent kaspad logs (last 50 lines, unfiltered)

```
ssh root@89.167.32.239 "journalctl -u kaspad --no-pager -n 50"
```

Good for spotting errors, peer disconnects, or anything unexpected. Common noise to ignore:
- `P2P protocol version mismatch` = old peers trying to connect, harmless.
- `timeout expired, disconnecting from peer` = normal peer churn.

---

## 7. Restart kaspad (if needed)

```
ssh root@89.167.32.239 "sudo systemctl restart kaspad"
```

Then wait ~10s and run the alive check (#3) to confirm it came back.

---

## 8. Tail logs live (watch sync in real time)

```
ssh root@89.167.32.239 "journalctl -u kaspad -f"
```

Ctrl+C to stop. Useful for watching IBD progress or debugging a problem in real time.
