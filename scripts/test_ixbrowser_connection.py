#!/usr/bin/env python3
"""
Test script to verify the connection to a local ixBrowser API service and list active profiles.
This script checks the API base URL, queries the list of opened profiles,
and verifies if they expose a working Chrome DevTools Protocol debugging interface
by checking the command lines of running browser processes.
"""

import argparse
import json
import os
import sys
import re
import subprocess
import urllib.error
import urllib.request


def http_post(url, data_dict=None):
    """Perform an HTTP POST request with JSON payload."""
    payload = json.dumps(data_dict or {}).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    with urllib.request.urlopen(req, timeout=5) as response:
        return response.read().decode("utf-8")


def http_get(url):
    """Perform an HTTP GET request and return the string response."""
    req = urllib.request.Request(url)
    with urllib.request.urlopen(req, timeout=2) as response:
        return response.read().decode("utf-8")


def get_running_chrome_ports():
    """Scan running chrome processes to find active ixBrowser profile ports."""
    ports = {}
    try:
        # Use Get-CimInstance on Windows to query command line arguments of running chrome processes
        cmd = ["powershell", "-NoProfile", "-Command",
               "Get-CimInstance Win32_Process -Filter \"name = 'chrome.exe'\" | Where-Object { $_.CommandLine -like '*--protected-userid=*' } | Select-Object -ExpandProperty CommandLine"]
        out = subprocess.check_output(cmd, stderr=subprocess.DEVNULL).decode('utf-8', errors='ignore')
        for line in out.splitlines():
            line = line.strip()
            if not line:
                continue
            if '--protected-userid=' in line and '--remote-debugging-port=' in line:
                uid_match = re.search(r'--protected-userid=(\d+)', line)
                port_match = re.search(r'--remote-debugging-port=(\d+)', line)
                if uid_match and port_match:
                    ports[uid_match.group(1)] = port_match.group(1)
    except Exception:
        pass
    return ports


def resolve_ws_url(address):
    """
    Attempt to connect to the debugging address and fetch the WebSocket Debugger URL.
    Checks the /json/version endpoint.
    """
    if address.startswith("ws://") or address.startswith("wss://"):
        return address

    if address.startswith("http://") or address.startswith("https://"):
        if address.endswith("/json/version"):
            url = address
        elif address.endswith("/"):
            url = f"{address}json/version"
        else:
            url = f"{address}/json/version"
    else:
        url = f"http://{address}/json/version"

    try:
        resp_text = http_get(url)
        data = json.loads(resp_text)
        return data.get("webSocketDebuggerUrl")
    except Exception as e:
        return f"Error: {e}"


def main():
    parser = argparse.ArgumentParser(
        description="Verify ixBrowser API connection and list active/opened profiles."
    )
    parser.add_argument(
        "--url", "-u",
        help="Base URL of the ixBrowser API (default: http://127.0.0.1:53200)"
    )
    args = parser.parse_args()

    # Determine base URL: command-line arg -> environment var -> default
    api_url = args.url
    if not api_url:
        api_url = os.environ.get("IXBROWSER_API_URL")
    if not api_url:
        api_url = "http://127.0.0.1:53200"

    # Normalize the base URL
    if not api_url.endswith("/"):
        api_url += "/"
    if "/api/v2" not in api_url:
        api_url += "api/v2/"

    print("=" * 60)
    print(f"Testing ixBrowser connection at: {api_url}")
    print("=" * 60)

    endpoint = f"{api_url}profile-opened-list"
    print(f"Querying opened profiles endpoint: {endpoint}")

    try:
        response_text = http_post(endpoint, {})
        response = json.loads(response_text)
    except urllib.error.URLError as e:
        print(f"\n[ERROR] Connection failed: {e}")
        print("\nPlease make sure that:")
        print(" 1. ixBrowser is currently running on this computer.")
        print(" 2. Local API server is enabled in ixBrowser (default port is 53200).")
        print(f" 3. The API port matches the tested address: {api_url}")
        sys.exit(1)
    except Exception as e:
        print(f"\n[ERROR] Unexpected error: {e}")
        sys.exit(1)

    error_obj = response.get("error") or {}
    code = error_obj.get("code")
    message = error_obj.get("message") or "No message"
    print(f"API Response Code: {code}")
    print(f"API Response Message: {message}")

    if code not in (0, 200):
        print(f"\n[ERROR] ixBrowser API returned an error code: {code}")
        print(f"Details: {message}")
        sys.exit(1)

    profiles = response.get("data")

    if not isinstance(profiles, list) or not profiles:
        print("\n[SUCCESS] Connected to ixBrowser API, but no profiles are currently open.")
        print("To test further, please open a profile in ixBrowser and run this script again.")
        sys.exit(0)

    print("\nScanning active processes to resolve debugging ports...")
    running_ports = get_running_chrome_ports()

    print(f"\nFound {len(profiles)} opened profile(s) in ixBrowser list:")
    print("-" * 60)

    for i, profile in enumerate(profiles):
        p_id = str(profile.get("profile_id") or profile.get("profileId") or profile.get("id") or f"unknown-{i}")
        
        # Get profile's name from profile-list
        p_name = f"IxBrowserProfile-{p_id}"
        try:
            list_text = http_post(f"{api_url}profile-list", {"profile_id": int(p_id) if p_id.isdigit() else p_id})
            list_resp = json.loads(list_text)
            if list_resp.get("error", {}).get("code") in (0, 200):
                list_data = list_resp.get("data", {}).get("data", [])
                if list_data:
                    p_name = list_data[0].get("name") or p_name
        except Exception:
            pass

        print(f"Profile #{i+1}:")
        print(f"  Name:   {p_name}")
        print(f"  ID:     {p_id}")

        port = running_ports.get(p_id)
        if port:
            print(f"  Status: Active (Running)")
            print(f"  Port:   {port}")
            print("  Checking CDP Debugging Interface...")
            resolved_ws = resolve_ws_url(f"127.0.0.1:{port}")
            if resolved_ws.startswith("ws://") or resolved_ws.startswith("wss://"):
                print(f"    [OK] WebSocket Debugger URL: {resolved_ws}")
            else:
                print(f"    [WARNING] Could not get WebSocket URL: {resolved_ws}")
                print("              Is the profile fully loaded and browser active?")
        else:
            print(f"  Status: Inactive (Closed or stuck state in ixBrowser)")

        print("-" * 60)

    print("\n[SUCCESS] ixBrowser integration test completed.")


if __name__ == "__main__":
    main()
