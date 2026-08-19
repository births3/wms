#!/usr/bin/env python3
"""模拟 ERP 确认 if_out_message（pending → acked）。

用法：
  python3 scripts/h8_erp_interface_sync/ack_if_out.py --all
  python3 scripts/h8_erp_interface_sync/ack_if_out.py --id <uuid>
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--all", action="store_true", help="确认全部 pending")
    parser.add_argument("--id", help="单条 if_out_message id")
    parser.add_argument(
        "--container",
        default=os.environ.get("H8_MSSQL_CONTAINER", "wms-mssql-erp-if"),
    )
    parser.add_argument(
        "--password",
        default=os.environ.get("H8_MSSQL_PASSWORD", "Wms_Erp_If_Dev_2026!"),
    )
    parser.add_argument(
        "--ack-ref",
        default="erp-sim-ack",
        help="写回 erp_ack_ref",
    )
    args = parser.parse_args(argv)
    if not args.all and not args.id:
        print("need --all or --id", file=sys.stderr)
        return 2
    if args.all:
        where = "sync_status = N'pending'"
    else:
        where = f"id = '{args.id}'"
    sql = f"""
SET NOCOUNT ON;
UPDATE dbo.if_out_message
   SET sync_status = N'acked',
       erp_ack_ref = N'{args.ack_ref}',
       updated_at = SYSUTCDATETIME()
 WHERE {where};
SELECT @@ROWCOUNT;
"""
    cmd = [
        "docker",
        "exec",
        "-i",
        args.container,
        "/opt/mssql-tools18/bin/sqlcmd",
        "-S",
        "localhost",
        "-U",
        "sa",
        "-P",
        args.password,
        "-C",
        "-b",
        "-d",
        "wms_erp_if",
        "-Q",
        sql,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        print(proc.stderr or proc.stdout, file=sys.stderr)
        return 1
    print(proc.stdout.strip() or "ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
