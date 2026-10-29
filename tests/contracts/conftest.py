import tomllib
from pathlib import Path

import pytest

CONFIG_PATH = Path(__file__).parent / "contracts.local.toml"

@pytest.fixture(scope="session")
def raw_config():
    if not CONFIG_PATH.exists():
        pytest.skip(f"Contract test configuration not found at {CONFIG_PATH}")
    with open(CONFIG_PATH, "rb") as f:
        return tomllib.load(f)

@pytest.fixture(scope="session")
def general_config(raw_config):
    return raw_config.get("general", {})

@pytest.fixture(scope="session")
def dynu_config(raw_config, general_config):
    cfg = raw_config.get("dynu", {})
    if not cfg.get("enabled", False):
        pytest.skip("Dynu contract tests are disabled in contracts.local.toml")
    return {
        **{k: v.strip() if isinstance(v, str) else v for k, v in cfg.items()},
        "test_ipv4": str(general_config.get("test_ipv4", "198.51.100.42")).strip(),
        "test_ipv6": str(general_config.get("test_ipv6", "2001:db8::42")).strip(),
    }


@pytest.fixture(scope="session")
def duckdns_config(raw_config, general_config):
    cfg = raw_config.get("duckdns", {})
    if not cfg.get("enabled", False):
        pytest.skip("DuckDNS contract tests are disabled in contracts.local.toml")
    return {
        **{k: v.strip() if isinstance(v, str) else v for k, v in cfg.items()},
        "test_ipv4": str(general_config.get("test_ipv4", "198.51.100.42")).strip(),
        "test_ipv6": str(general_config.get("test_ipv6", "2001:db8::42")).strip(),
    }
