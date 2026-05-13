#!/usr/bin/env python3
"""
Example custom agent for Bacon workflow.
This demonstrates the CLI worker contract for external agents.
"""

import json
import sys
import argparse
import time
from typing import Dict, Any


def process_observer(prompt: str) -> Dict[str, Any]:
    """Observer role: Find improvement opportunities."""
    return {
        "status": "ok",
        "description": f"Found improvement opportunity based on prompt: {prompt[:100]}...",
        "summary": "Observer identified code quality improvement",
    }


def process_strategist(prompt: str) -> Dict[str, Any]:
    """Strategist role: Create implementation plan."""
    return {
        "status": "ok",
        "description": "Created detailed implementation plan",
        "summary": "Strategist planned refactoring approach",
        "spec_path": "docs/specs/_active/0001-example-refactor",
    }


def process_coder(prompt: str) -> Dict[str, Any]:
    """Coder role: Implement changes."""
    return {
        "status": "ok",
        "description": "Implemented code changes successfully",
        "summary": "Coder completed implementation",
    }


def process_auditor(prompt: str) -> Dict[str, Any]:
    """Auditor role: Review and validate changes."""
    return {
        "status": "ok",
        "description": "Audited changes and found them acceptable",
        "summary": "Auditor approved implementation",
    }


def main():
    parser = argparse.ArgumentParser(
        description="Bacon Custom Agent Example",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s -p "fix clippy warnings" --role observer
  %(prog)s -p "refactor error handling" --role strategist
        """,
    )
    parser.add_argument(
        "-p", "--prompt", required=True, help="User prompt describing the task"
    )
    parser.add_argument(
        "--role",
        required=True,
        choices=["observer", "strategist", "coder", "auditor"],
        help="Role to execute",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="Enable verbose logging"
    )

    args = parser.parse_args()

    # Log to stderr (as per contract)
    if args.verbose:
        print(
            f"[DEBUG] Processing role={args.role}, prompt={args.prompt[:50]}...",
            file=sys.stderr,
        )

    try:
        # Route to appropriate role handler
        if args.role == "observer":
            result = process_observer(args.prompt)
        elif args.role == "strategist":
            result = process_strategist(args.prompt)
        elif args.role == "coder":
            result = process_coder(args.prompt)
        elif args.role == "auditor":
            result = process_auditor(args.prompt)
        else:
            result = {
                "status": "error",
                "description": f"Unknown role: {args.role}",
            }

        # Simulate processing time
        time.sleep(0.5)

        # Output JSON to stdout (as per contract)
        print(json.dumps(result, indent=2))

        # Log completion
        if args.verbose:
            print(f"[DEBUG] Completed with status: {result['status']}", file=sys.stderr)

        return 0 if result.get("status") == "ok" else 1

    except Exception as e:
        # Handle errors gracefully
        error_result = {
            "status": "error",
            "description": f"Agent failed: {str(e)}",
        }
        print(json.dumps(error_result, indent=2), file=sys.stdout)
        print(f"[ERROR] Agent failed: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())