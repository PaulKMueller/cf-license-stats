# cf-license-stats

A dashboard to track copier template rollout progress.

## Installation

You can install the package in development mode using:

```bash
git clone https://github.com/PaulKMueller/cf-license-stats
cd cf-license-stats

pixi run pre-commit-install
pixi run postinstall
pixi run test
```

## Usage

There are two main commands you can use to interact with the dashboard.

### 1. Rebuild

```bash
pixi run rebuild
```

This runs the `main.rs` which fetches all data upon which the dashboard is based, overwriting any preexisting data.

### 2. Stream

```bash
pixi run stream
```

The dashboard is built using [Streamlit](https://github.com/streamlit/streamlit).
This `pixi` task runs the `app.py` which constitutes the endpoint to our streamlit application.
Running the application in this way is useful for debugging purposes.
