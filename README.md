# FINFILES

FINFILES is a professional, cross-platform SEC EDGAR data access app. 
It fetches and processes SEC filings (such as 10-K and 10-Q ) asynchronously for speed and responsiveness, and includes advanced filtering, export, and audit logging features.
All data access is performed using only free, public SEC EDGAR sources, with robust input validation and role-based access control for enterprise security.

---
## Getting Started

### Requirements

- Rust 
- GTK 3+ development libraries 
- Required dependencies 

### Running the Application

1. Clone the repository:
   ```sh
   git clone https://github.com/your-org/finfiles.git
   cd finfiles
   ```
2. Install dependencies:
   - **Windows:** Ensure GTK is installed and available in your `PATH`.
   - **macOS/Linux:** Use your package manager to install GTK 3.
3. Build and run:
   ```sh
   cargo run 
   ```
---

## Features

- **SEC EDGAR Data Access**  
  Fetch and analyze public SEC filings (10-K, 10-Q, etc.) for any US-listed company.

- **Modern GTK GUI**  
  Native, high-contrast, accessible interface for Windows, macOS, and Linux.

- **Advanced Filtering**  
  Filter filings by form type, date range, and more.

- **Export Options**  
  Export filings as CSV, PDF, or JSON.

- **Real-Time Updates**  
  Live filing updates via WebSocket.

- **AI/ML Analytics**  
  Modular, pluggable AI engine for summaries, forecasts, anomaly detection, and more.

- **Audit Logging & Security**  
  All actions are logged; role-based access control and input sanitization enforced.

- **Accessibility**  
  Full keyboard navigation, screen reader support, and high-contrast color scheme.

- **No Proprietary Data**  
  100% free, open, and independent.

---

## Architecture

- **Rust Modular Codebase**  
  Separate modules for backend (EDGAR API), security, analytics, export, filters, websocket, AI, data ingestion, and chat UI.

- **Asynchronous Operations**  
  Uses `tokio` and `glib` for scalable, responsive data fetching and UI updates.

- **AI/ML Engine**  
  Pluggable backends (independent FINFILES AI, ONNX, remote LLM, custom models).

- **Data Ingestion**  
  Modular support for SEC EDGAR, with future extensibility for Yahoo Finance, Alpha Vantage, etc.

- **Export & Logging**  
  All data access and export actions are logged for auditability.

---

## Usage

- Enter ticker(s) in the input field or upload a CSV.
- Fetch SEC filings with the button or <kbd>Ctrl+F</kbd>.
- Filter results as needed.
- Export data with <kbd>Ctrl+E</kbd>.
- Open AI Chat for advanced analysis, summaries, forecasts, and anomaly detection.
- Visualize trends in the chart area.
- Receive real-time updates as new filings are published.

---

## Security & Privacy

- **Authentication:** OAuth2/OIDC login, role-based access control.
- **Audit Logging:** All data access and export actions are logged.
- **No proprietary data:** Only public, open SEC data is used.

---

## Extensibility

- Add new data sources (e.g., Yahoo Finance) by implementing a new data ingestion module.
- Plug in new AI/ML models (ONNX, remote LLM, or your own) via the AI engine abstraction.
- Customize the UI with additional filters, charts, or export formats.

---
## UI Tabs Overview

FINFILES provides a streamlined, tabbed interface to organize all major features for efficient workflow. Below is a summary of each main tab and its core functionality:

### 1. **Dashboard**
- **Key Features:**  
  - Ticker input field  
  - Fetch and Export buttons  
  - Real-time status label  
  - Quick access to filters  

### 2. **Filings Table**
  - TreeView table: Form, Date, Document (clickable), Company, Filing Type, AI Analysis  
  - Pagination with "Load More"  
  - Real-time updates via WebSocket  
  - Clickable document links open filings in your browser  

### 3. **Filters**
  - Form type dropdown  
  - Date range selectors  
  - Advanced filter options  

### 4. **Charts & Analytics**
  - Interactive charts (e.g., filing frequency, revenue trends)  
  - Export chart data  
  - AI-powered insights  

### 5. **FINFILES AI Chat** 
  - Ask questions about SEC data (e.g., "Summarize", "Forecast", "Show revenue")  
  - Select AI backend (FINFILES AI, ONNX, Remote LLM, Custom Model)  
  - View chat history and export results  
  - Upload custom AI models  

### 6. **Export**  
  - Export as CSV, PDF, or JSON  
  - Audit log of export actions  

### 7. **Settings & Security** 
  - User authentication (OAuth2/OIDC)  
  - Role-based access control  
  - Theme and accessibility options  
---
## Disclaimer

This project is provided for educational and research purposes only. It is not financial advice, nor an invitation to trade or invest.
The author does not guarantee the accuracy, completeness, or profitability of this trading system. Use of this code in live or paper trading environments is at your own risk.
Trading financial instruments such as stocks, options, or derivatives involves significant risk of loss and may not be suitable for all investors.
You are solely responsible for any decisions or trades you make. Before using this system, consult with a qualified financial advisor and ensure compliance with your local regulations and your broker’s terms of service.
The author is not liable for any damages, financial losses, or legal issues resulting from the use of this codebase.
