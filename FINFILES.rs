// Imports and Dependencies
mod backend;
mod security;
mod analytics;
mod export;
mod filters;
mod websocket;
mod ai;
mod data_ingestion;
mod chat_ui;

use std::sync::Arc;
use backend::{SecEdgarApi, AppState, FilingRecord};
use security::{sanitize_ticker, AuthManager, RBACRole};
use export::export_filings;
use filters::FilterPane;
use websocket::start_realtime_updates;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Button, Entry, ScrolledWindow, Box as GtkBox, Orientation,
    Label, Adjustment, Spinner, TreeView, TreeViewColumn, ListStore, CellRendererText,
    CssProvider, StyleContext, Image, Align,
};
use glib::{self, clone, Type};
use log::{info, error};
use pango;
use polars::prelude::*;
use crate::ai::{FinfilesAI, OnnxAIModule, RemoteLLMAIModule, FinancialAIModule, CustomModelAIModule};
use crate::data_ingestion::FinancialDataLoader;
use crate::chat_ui::FinancialAIChatApp;
use crate::error::*;

// Unified Main Window
fn build_main_window(app: &Application, state: Arc<AppState>, auth: Arc<AuthManager>, ai_modules: Vec<Arc<dyn FinancialAIModule>>, ai_data: Option<DataFrame>, audit_log_path: std::path::PathBuf, username: String) -> ApplicationWindow {
    let window = ApplicationWindow::new(app);
    window.set_title("AA SEC EDGAR + FINFILES AI: Professional Financial Data & AI Platform");
    window.set_default_size(1400, 900);
    window.set_resizable(true);

    // Modern, high-contrast CSS
    let provider = CssProvider::new();
    provider
        .load_from_data(
            br#"
                window { background: #181c20; }
                box#main_vbox { background: #23272e; border-radius: 12px; padding: 24px; }
                box#header_hbox { background: #23272e; border-radius: 8px; margin-bottom: 12px; padding: 12px 8px; }
                button {
                    background: linear-gradient(90deg, #1976d2 0%, #1565c0 100%);
                    color: #fff;
                    font: bold 16px 'Segoe UI', Arial, sans-serif;
                    border-radius: 6px;
                    min-width: 120px;
                    min-height: 36px;
                    margin: 0 4px;
                    box-shadow: 0 2px 8px rgba(25,118,210,0.08);
                    transition: background 0.2s;
                }
                button:hover, button:focus {
                    background: linear-gradient(90deg, #2196f3 0%, #1976d2 100%);
                    color: #fff;
                    outline: 2px solid #90caf9;
                }
                entry {
                    font: 16px 'Segoe UI', Arial, sans-serif;
                    background: #1a1d22;
                    color: #e3eaf3;
                    border-radius: 6px;
                    border: 1.5px solid #1976d2;
                    padding: 8px 12px;
                    min-width: 320px;
                    margin-right: 8px;
                }
                entry:focus {
                    border: 2px solid #42a5f5;
                    background: #23272e;
                }
                label {
                    font: 15px 'Segoe UI', Arial, sans-serif;
                    color: #b0bec5;
                }
                treeview {
                    font: 15px 'Segoe UI', Arial, sans-serif;
                    background: #23272e;
                    color: #e3eaf3;
                    border-radius: 8px;
                    border: 1.5px solid #1976d2;
                }
                treeview row:selected, treeview row:selected:focus {
                    background: #1976d2;
                    color: #fff;
                }
                treeview header {
                    background: #1565c0;
                    color: #fff;
                    font-weight: bold;
                    font-size: 15px;
                }
                spinner { color: #42a5f5; }
                #status_label {
                    color: #90caf9;
                    font: italic 15px 'Segoe UI', Arial, sans-serif;
                    margin-top: 8px;
                }
                #load_more_button {
                    background: linear-gradient(90deg, #43a047 0%, #388e3c 100%);
                    color: #fff;
                    font-weight: bold;
                }
                #load_more_button:hover, #load_more_button:focus {
                    background: linear-gradient(90deg, #66bb6a 0%, #43a047 100%);
                    outline: 2px solid #a5d6a7;
                }
                #export_button {
                    background: linear-gradient(90deg, #fbc02d 0%, #f9a825 100%);
                    color: #23272e;
                    font-weight: bold;
                }
                #export_button:hover, #export_button:focus {
                    background: linear-gradient(90deg, #ffe082 0%, #fbc02d 100%);
                    outline: 2px solid #ffe082;
                }
                #ai_chat_button {
                    background: linear-gradient(90deg, #8e24aa 0%, #3949ab 100%);
                    color: #fff;
                    font-weight: bold;
                }
                #ai_chat_button:hover, #ai_chat_button:focus {
                    background: linear-gradient(90deg, #ba68c8 0%, #7986cb 100%);
                    outline: 2px solid #b39ddb;
                }
            "#
        )
        .expect("Failed to load CSS");
    StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Main vertical layout
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_widget_name("main_vbox");

    // Header: App logo and title
    let header_hbox = GtkBox::new(Orientation::Horizontal, 10);
    header_hbox.set_widget_name("header_hbox");
    let logo = Image::from_icon_name(Some("emblem-documents"), gtk::IconSize::Dialog);
    logo.set_pixel_size(48);
    let title_label = Label::new(Some("AA SEC EDGAR + FINFILES AI"));
    title_label.set_markup("<span size='xx-large' weight='bold' foreground='#42a5f5'>AA SEC EDGAR <span foreground='#fff'>+ FINFILES AI</span></span>");
    title_label.set_halign(Align::Start);
    header_hbox.pack_start(&logo, false, false, 0);
    header_hbox.pack_start(&title_label, false, false, 0);

    // Open Data Only badge
    let open_data_label = Label::new(Some("100% Free & Open SEC Data + Independent AI"));
    open_data_label.set_markup("<span background='#43a047' foreground='#fff' weight='bold' size='large' rise='2000'> 100% Free & Open SEC Data + Independent AI </span>");
    open_data_label.set_halign(Align::End);
    header_hbox.pack_end(&open_data_label, false, false, 0);

    vbox.pack_start(&header_hbox, false, false, 0);

    // Ticker input and filter pane
    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    let ticker_entry = Entry::new();
    ticker_entry.set_placeholder_text(Some("Enter Ticker(s) (or upload CSV)"));
    ticker_entry.set_tooltip_text(Some("Type a stock ticker, comma-separated, or upload a CSV"));
    ticker_entry.set_width_chars(24);

    let fetch_button = Button::new_with_label("Fetch SEC Filings (Ctrl+F)");
    fetch_button.set_widget_name("fetch_button");
    fetch_button.set_tooltip_text(Some("Fetch latest SEC filings for the given ticker(s)"));

    let export_button = Button::new_with_label("Export (Ctrl+E)");
    export_button.set_widget_name("export_button");
    export_button.set_tooltip_text(Some("Export filings as CSV, PDF, or JSON"));

    let ai_chat_button = Button::new_with_label("Open FINFILES AI Chat");
    ai_chat_button.set_widget_name("ai_chat_button");
    ai_chat_button.set_tooltip_text(Some("Analyze SEC data with FINFILES AI (chat, summary, forecast, anomaly, etc.)"));

    let filter_pane = FilterPane::new();
    hbox.pack_start(&ticker_entry, true, true, 0);
    hbox.pack_start(&fetch_button, false, false, 0);
    hbox.pack_start(&export_button, false, false, 0);
    hbox.pack_start(&ai_chat_button, false, false, 0);
    hbox.pack_start(&filter_pane.widget, false, false, 0);

    // Spinner (loading indicator)
    let spinner = Spinner::new();
    spinner.set_halign(Align::End);
    spinner.set_valign(Align::Center);
    hbox.pack_start(&spinner, false, false, 0);

    vbox.pack_start(&hbox, false, false, 0);

    // Output area: TreeView for filings
    let scrolled = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
    scrolled.set_shadow_type(gtk::ShadowType::EtchedIn);
    scrolled.set_min_content_height(350);
    scrolled.set_min_content_width(900);

    let filings_store = ListStore::new(&[
        Type::STRING, // Form
        Type::STRING, // Date
        Type::STRING, // Document
        Type::STRING, // Document URL
        Type::STRING, // Company Name
        Type::STRING, // Filing Type
        Type::STRING, // Sentiment/AI
    ]);
    let filings_view = TreeView::with_model(&filings_store);
    filings_view.set_headers_visible(true);
    filings_view.set_search_column(0);
    filings_view.set_tooltip_column(2);
    filings_view.set_grid_lines(gtk::TreeViewGridLines::Both);

    // Add columns with icons 
    let columns = [
        ("Form", 0, Some("text-x-generic")),
        ("Date", 1, Some("x-office-calendar")),
        ("Document", 2, Some("document-open")),
        ("Company", 4, Some("emblem-people")),
        ("Filing Type", 5, Some("view-list-details")),
        ("AI Analysis", 6, Some("system-search")),
    ];
    for (title, idx, icon_name) in columns.iter() {
        let renderer = CellRendererText::new();
        let column = TreeViewColumn::new();
        if let Some(icon) = icon_name {
            let icon_img = Image::from_icon_name(Some(icon), gtk::IconSize::Menu);
            column.set_widget(Some(&icon_img));
        }
        column.set_title(title);
        column.pack_start(&renderer, true);
        column.add_attribute(&renderer, "text", *idx);
        filings_view.append_column(&column);
    }

    // Make Document column clickable
    if let Some(doc_col) = filings_view.get_column(2) {
        if let Some(cell) = doc_col.get_cells().get(0) {
            if let Ok(renderer) = cell.clone().downcast::<CellRendererText>() {
                renderer.set_property_underline(pango::Underline::Single);
                renderer.set_property_foreground(Some("#42a5f5"));
                renderer.set_property_editable(false);
            }
        }
    }

    // Status label
    let status_label = Label::new(Some("Ready."));
    status_label.set_widget_name("status_label");
    status_label.set_halign(Align::Start);

    vbox.pack_start(&scrolled, true, true, 0);
    vbox.pack_start(&status_label, false, false, 0);

    scrolled.add(&filings_view);

    // Pagination: Load more
    let load_more_button = Button::new_with_label("Load More");
    load_more_button.set_widget_name("load_more_button");
    load_more_button.set_tooltip_text(Some("Load more filings"));
    load_more_button.set_sensitive(false);
    vbox.pack_start(&load_more_button, false, false, 0);

    // Chart area for data visualization
    let chart_area = analytics::FilingTrendsChart::new();
    vbox.pack_start(&chart_area.widget, false, false, 0);

    // Real-time updates via WebSocket
    start_realtime_updates(state.clone(), filings_store.clone(), status_label.clone());

    // Functional Event Handlers
    let display_filings = {
        let filings_store = filings_store.clone();
        let status_label = status_label.clone();
        let load_more_button = load_more_button.clone();
        let chart_area = chart_area.clone();
        let state = state.clone();
        move |records: &[FilingRecord], append: bool| {
            if !append {
                filings_store.clear();
            }
            let mut shown = 0;
            for rec in records {
                filings_store.insert_with_values(
                    None,
                    &[0, 1, 2, 3, 4, 5, 6],
                    &[
                        &rec.form,
                        &rec.date,
                        &rec.document,
                        &rec.document_url,
                        &rec.company_name,
                        &rec.filing_type,
                        &rec.ai_summary,
                    ],
                );
                shown += 1;
            }
            if shown == 0 && !append {
                status_label.set_text("No recent filings found.");
            } else {
                status_label.set_text(&format!("Showing {} filings.", shown));
            }
            chart_area.update(records);
            load_more_button.set_sensitive(state.has_more_filings());
        }
    };

    // Fetch filings logic 
    let fetch_and_display = {
        let state = state.clone();
        let filings_store = filings_store.clone();
        let status_label = status_label.clone();
        let spinner = spinner.clone();
        let load_more_button = load_more_button.clone();
        let display_filings = display_filings.clone();
        let auth = auth.clone();
        let filter_pane = filter_pane.clone();

        move |tickers: Vec<String>, append: bool| {
            let state = state.clone();
            let filings_store = filings_store.clone();
            let status_label = status_label.clone();
            let spinner = spinner.clone();
            let load_more_button = load_more_button.clone();
            let display_filings = display_filings.clone();
            let auth = auth.clone();
            let filter_pane = filter_pane.clone();

            spinner.start();
            status_label.set_text("Fetching SEC filings...");
            load_more_button.set_sensitive(false);

            // RBAC: Only allow access to permitted tickers
            let user = auth.current_user();
            let allowed_tickers = auth.filter_allowed_tickers(&user, &tickers);

            // Use tokio for async, scalable fetch
            glib::MainContext::default().spawn_local(async move {
                // Fetch from public SEC EDGAR data
                audit_log(&user, "fetch_filings", &allowed_tickers);
                match state.api.fetch_multiple_filings(allowed_tickers, filter_pane.filters()).await {
                    Ok(records) => {
                        state.set_filings(records.clone());
                        display_filings(&records, append);
                        status_label.set_text("Filings loaded.");
                    }
                    Err(e) => {
                        error!("Error fetching/displaying filings: {}", e);
                        status_label.set_text(&format!("Error: {}", e));
                    }
                }
                spinner.stop();
            });
        }
    };

    // Keyboard accessibility: Enter triggers fetch, Ctrl+F/Ctrl+E shortcuts
    let fetch_button_clone = fetch_button.clone();
    ticker_entry.connect_activate(clone!(@strong fetch_button_clone => move |_| {
        fetch_button_clone.clicked();
    }));

    // Keyboard shortcuts
    let fetch_and_display_clone = fetch_and_display.clone();
    let export_button_clone = export_button.clone();
    window.connect_key_press_event(move |_, event| {
        let ctrl = event.get_state().contains(gdk::ModifierType::CONTROL_MASK);
        match event.get_keyval() {
            gdk::keys::constants::F if ctrl => {
                fetch_button_clone.clicked();
                Inhibit(true)
            }
            gdk::keys::constants::E if ctrl => {
                export_button_clone.clicked();
                Inhibit(true)
            }
            _ => Inhibit(false),
        }
    });

    // Fetch button click
    {
        let ticker_entry = ticker_entry.clone();
        let fetch_and_display = fetch_and_display.clone();
        let status_label = status_label.clone();
        fetch_button.connect_clicked(move |_| {
            let input = ticker_entry.text().to_string();
            if input.trim().is_empty() {
                status_label.set_text("Please enter a ticker symbol or upload a CSV.");
                return;
            }
            let tickers: Vec<String> = input
                .split(',')
                .map(|t| sanitize_ticker(t))
                .filter(|t| !t.is_empty())
                .collect();
            if tickers.is_empty() {
                status_label.set_text("No valid tickers found.");
                return;
            }
            fetch_and_display(tickers, false);
        });
    }

    // Export button click
    {
        let state = state.clone();
        let status_label = status_label.clone();
        export_button.connect_clicked(move |_| {
            match export_filings(&state.get_filings()) {
                Ok(path) => status_label.set_text(&format!("Exported to {}", path)),
                Err(e) => status_label.set_text(&format!("Export failed: {}", e)),
            }
        });
    }

    // Load more button click
    {
        let state = state.clone();
        let display_filings = display_filings.clone();
        let load_more_button = load_more_button.clone();
        let status_label = status_label.clone();
        load_more_button.connect_clicked(move |_| {
            if let Some(records) = state.load_more_filings() {
                display_filings(&records, true);
            } else {
                status_label.set_text("No more filings to load.");
            }
        });
    }

    // Clickable document links
    filings_view.connect_row_activated(move |view, path, _| {
        if let Some(model) = view.get_model() {
            if let Some(iter) = model.get_iter(path) {
                let url: Option<String> = model.get_value(&iter, 3).get().ok();
                if let Some(url) = url {
                    if let Err(e) = open::that(url) {
                        error!("Failed to open browser: {}", e);
                    }
                }
            }
        }
    });

    // Accessibility (focus indicators, tooltips, keyboard navigation)
    fetch_button.set_can_focus(true);
    ticker_entry.set_can_focus(true);
    filings_view.set_can_focus(true);
    load_more_button.set_can_focus(true);
    export_button.set_can_focus(true);
    ai_chat_button.set_can_focus(true);

    // Add tooltips for accessibility
    fetch_button.set_tooltip_text(Some("Fetch filings for the entered ticker symbol(s)"));
    ticker_entry.set_tooltip_text(Some("Enter stock ticker(s) or upload a CSV"));
    filings_view.set_tooltip_text(Some("List of recent SEC filings. Double-click a row to open the document."));
    load_more_button.set_tooltip_text(Some("Load more filings for this ticker"));
    export_button.set_tooltip_text(Some("Export filings as CSV, PDF, or JSON"));
    ai_chat_button.set_tooltip_text(Some("Open FINFILES AI chat for advanced analysis"));

    // Keyboard navigation: set tab order
    fetch_button.set_focus_on_click(true);
    load_more_button.set_focus_on_click(true);
    export_button.set_focus_on_click(true);
    ai_chat_button.set_focus_on_click(true);

    // Enable dark mode if available (GTK 3+ uses system theme by default)
    #[cfg(feature = "v3_16")]
    {
        if let Some(settings) = gtk::Settings::get_default() {
            settings.set_property_gtk_application_prefer_dark_theme(true);
        }
    }

    // FINFILES AI Chat Integration
    let ai_modules_for_chat = ai_modules.clone();
    let audit_log_path_for_chat = audit_log_path.clone();
    let username_for_chat = username.clone();
    let ai_data_for_chat = ai_data.clone();

    // Add FinfilesAI GUI section
    let finfiles_ai_box = GtkBox::new(Orientation::Vertical, 5);
    let finfiles_ai_title = Label::new(Some("FINFILES AI Analysis"));
    finfiles_ai_title.set_markup("<span size='large' weight='bold'>FINFILES AI Analysis</span>");
    finfiles_ai_box.append(&finfiles_ai_title);

    let finfiles_ai_output = TextView::new();
    finfiles_ai_output.set_editable(false);
    finfiles_ai_output.set_accessible_name(Some("FINFILES AI Output"));
    finfiles_ai_output.set_can_focus(true);
    let finfiles_ai_scroll = ScrolledWindow::builder()
        .child(&finfiles_ai_output)
        .min_content_height(200)
        .build();
    finfiles_ai_box.append(&finfiles_ai_scroll);

    let analyze_button = Button::with_label("Analyze with FinfilesAI");
    analyze_button.set_accessible_name(Some("Analyze Button"));
    analyze_button.set_can_focus(true);
    finfiles_ai_box.append(&analyze_button);

    // Connect analyze button to trigger FinfilesAI analysis
    let finfiles_ai_output_clone = finfiles_ai_output.clone();
    let ai_data_for_analyze = ai_data_for_chat.clone();
    analyze_button.connect_clicked(move |_| {
        if let Some(df) = &ai_data_for_analyze {
            let finfiles_ai = FinfilesAI::new().unwrap();
            let query = "Analyze the data";
            let output_buffer = finfiles_ai_output_clone.buffer().unwrap();
            output_buffer.set_text("");
            glib::MainContext::default().spawn_local(async move {
                match finfiles_ai.analyze(df, query).await {
                    Ok(result) => {
                        output_buffer.set_text(&result);
                    }
                    Err(e) => {
                        output_buffer.set_text(&format!("Error: {}", e));
                    }
                }
            });
        } else {
            finfiles_ai_output_clone.buffer().unwrap().set_text("No data available for analysis.");
        }
    });

    // Add FinfilesAI section to the main window
    vbox.append(&finfiles_ai_box);

    // Application Entry Point
    fn main() {
        // Initialize logging, monitoring, and security
        logging::init();
        security::init_tls();
        let auth = Arc::new(AuthManager::new());

        // Authenticate user (OAuth2, OIDC, etc.)
        let user = auth.authenticate_user();
        if user.is_none() {
            eprintln!("Authentication failed. Exiting.");
            return;
        }
        let user = user.unwrap();

        // RBAC (Only allow access to permitted tickers/features)
        if !auth.has_role(&user, RBACRole::User) {
            eprintln!("Access denied. Contact support.");
            return;
        }

        // Start backend microservices (API, DB, cache, websocket, analytics)
        backend::start_services();

        // GTK Application
        let app = Application::new(
            Some("com.aa.sec_edgar_professional"),
            Default::default(),
        );

        let state = Arc::new(AppState::new(user.clone()));
        let auth_arc = auth.clone();

        app.connect_activate(move |app| {
            let window = build_main_window(app, state.clone(), auth_arc.clone());
            window.present();
        });

        app.run();
    }

    pub mod error {
        use thiserror::Error;

        #[derive(Error, Debug)]
        pub enum FinAIError {
            #[error("Network error: {0}")]
            Network(String),
            #[error("Ticker not found: {0}")]
            TickerNotFound(String),
            #[error("SEC data not found for ticker: {0}")]
            SecDataNotFound(String),
            #[error("Yahoo Finance data not found for ticker: {0}")]
            YahooDataNotFound(String),
            #[error("Data parsing error: {0}")]
            DataParsing(String),
            #[error("AI module error: {0}")]
            AIModule(String),
            #[error("Authentication error: {0}")]
            Auth(String),
            #[error("Unknown error: {0}")]
            Unknown(String),
            #[error("Custom model error: {0}")]
            CustomModel(String),
        }

        pub type Result<T> = std::result::Result<T, FinAIError>;
    }

    pub mod ai {
        use super::error::*;
        use polars::prelude::*;
        use async_trait::async_trait;
        use std::sync::Arc;

        // Trait for pluggable AI/ML backends.
        #[async_trait]
        pub trait FinancialAIModule: Send + Sync {
            // Analyze a DataFrame with a natural language query.
            async fn analyze(&self, df: &DataFrame, query: &str) -> Result<String>;
            fn backend_name(&self) -> &'static str;
        }

        // Independent FINFILES AI model (default, independent, no external dependencies)
        pub struct FinfilesAI;

        pub struct OnnxAIModule {
            pub model_name: String,
            session: Arc<Session>, 
        }

        impl OnnxAIModule {
            // Initialize the ONNX module with our own independent model.
            pub fn new() -> Result<Self, crate::error::FinAIError> {
                log::info!("FINFILES AI: Initializing INDEPENDENT ONNX backend (our own model, no external AI)...");

                // Path to our own ONNX model file (must exist and be trained by us)
                let model_path = Path::new("models/finfiles_independent.onnx");
                if !model_path.exists() {
                    return Err(crate::error::FinAIError::AIModule(
                        "Independent ONNX model file not found.".to_string(),
                    ));
                }

                // Create ONNX Runtime environment
                let environment = Environment::builder()
                    .with_name("finfiles_onnx_independent")
                    .with_log_level(LoggingLevel::Warning)
                    .build()
                    .map_err(|e| crate::error::FinAIError::AIModule(format!("ONNX env error: {e}")))?;

                // Load our own ONNX model
                let session = environment
                    .new_session_builder()
                    .map_err(|e| crate::error::FinAIError::AIModule(format!("ONNX session builder error: {e}")))?
                    .with_model_from_file(model_path)
                    .map_err(|e| crate::error::FinAIError::AIModule(format!("ONNX model load error: {e}")))?;

                Ok(Self {
                    model_name: "FinfilesIndependentAI".to_string(),
                    session: Arc::new(session),
                })
            }

            // Run inference using our own independent ONNX model.
            pub fn run_inference(&self, input: Vec<f32>) -> Result<Vec<f32>, crate::error::FinAIError> {
                use onnxruntime::ndarray::Array;
                use onnxruntime::ndarray::IxDyn;

                // Prepare input tensor (example: 1D input)
                let input_array = Array::from_shape_vec(IxDyn(&[1, input.len()]), input)
                    .map_err(|e| crate::error::FinAIError::AIModule(format!("Input shape error: {e}")))?;

                // Get input/output names
                let input_name = self.session.inputs[0].name.clone();
                let output_name = self.session.outputs[0].name.clone();

                // Run inference
                let outputs: Vec<OrtOwnedTensor<f32, _>> = self
                    .session
                    .run(vec![(input_name.as_str(), &input_array)])
                    .map_err(|e| crate::error::FinAIError::AIModule(format!("ONNX inference error: {e}")))?;

                // Extract output
                let output_tensor = outputs
                    .get(0)
                    .ok_or_else(|| crate::error::FinAIError::AIModule("No output from ONNX model".to_string()))?;

                Ok(output_tensor.iter().cloned().collect())
            }
        }
        pub struct OnnxAIModule;
        pub struct RemoteLLMAIModule;
        pub struct CustomModelAIModule {
            pub name: String,
        }

        impl FinfilesAI {
            pub fn new() -> Result<Self> {
                log::info!("FINFILES AI: Initializing independent FINFILES AI backend...");
                Ok(Self {})
            }
        }
        impl OnnxAIModule {
            pub fn new() -> Result<Self> {
                log::info!("FINFILES AI: Initializing ONNX backend...");
                Ok(Self {})
            }
        }
        impl RemoteLLMAIModule {
            pub fn new() -> Result<Self> {
                log::info!("FINFILES AI: Initializing Remote LLM backend...");
                Ok(Self {})
            }
        }
        impl CustomModelAIModule {
            pub fn new(name: String) -> Result<Self> {
                log::info!("FINFILES AI: Initializing custom model backend: {}", name);
                Ok(Self { name })
            }
        }

        #[async_trait]
        impl FinancialAIModule for FinfilesAI {
            async fn analyze(&self, df: &DataFrame, query: &str) -> Result<String> {
                let normalized_query = query.to_lowercase();

                // Show table/raw
                if normalized_query.contains("raw") || normalized_query.contains("table") {
                    return Ok(format!("SEC Data Table:\n{}", df));
                }

                // Summary of SEC data
                if normalized_query.contains("summarize") || normalized_query.contains("summary") {
                    let quarters = df.column("quarter").ok().and_then(|s| s.utf8().ok()).map(|s| s.len()).unwrap_or(0);
                    let mut summary_lines = Vec::new();
                    for col in df.get_columns() {
                        if let Ok(f64chunked) = col.f64() {
                            let sum: f64 = f64chunked.into_iter().flatten().sum();
                            let avg: f64 = if f64chunked.len() > 0 { sum / f64chunked.len() as f64 } else { 0.0 };
                            let most_recent = f64chunked.into_iter().flatten().last().unwrap_or(0.0);
                            summary_lines.push(format!(
                                "  • {}: Total = {:.2}B, Avg = {:.2}B, Most Recent = {:.2}B",
                                col.name(), sum, avg, most_recent
                            ));
                        }
                    }
                    return Ok(format!(
                        "Summary: {} quarters of SEC data loaded.\n{}",
                        quarters,
                        summary_lines.join("\n")
                    ));
                }

                // Time-series forecasting
                if normalized_query.contains("forecast") || normalized_query.contains("predict") {
                    // Naive forecast: last value as prediction for next period
                    let mut forecast_lines = Vec::new();
                    for col in df.get_columns() {
                        if let Ok(f64chunked) = col.f64() {
                            let last = f64chunked.into_iter().flatten().last().unwrap_or(0.0);
                            forecast_lines.push(format!(
                                "  • {}: Next period forecast (naive) = {:.2}B",
                                col.name(), last
                            ));
                        }
                    }
                    return Ok(format!(
                        "Time-Series Forecast (naive, last value):\n{}",
                        forecast_lines.join("\n")
                    ));
                }

                // Anomaly detection
                if normalized_query.contains("anomaly") || normalized_query.contains("outlier") {
                    let mut anomaly_lines = Vec::new();
                    for col in df.get_columns() {
                        if let Ok(f64chunked) = col.f64() {
                            let vals: Vec<f64> = f64chunked.into_iter().flatten().collect();
                            if vals.len() < 2 { continue; }
                            let mean = vals.iter().sum::<f64>() / vals.len() as f64;
                            let std = (vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64).sqrt();
                            for (i, v) in vals.iter().enumerate() {
                                if (*v - mean).abs() > 2.0 * std {
                                    anomaly_lines.push(format!(
                                        "  • {}: Anomaly detected at period {} (value = {:.2}B, mean = {:.2}B, std = {:.2}B)",
                                        col.name(), i + 1, v, mean, std
                                    ));
                                }
                            }
                        }
                    }
                    if anomaly_lines.is_empty() {
                        return Ok("No significant anomalies detected in the available metrics.".to_string());
                    }
                    return Ok(format!(
                        "Anomaly Detection Results:\n{}",
                        anomaly_lines.join("\n")
                    ));
                }

                // Show quarters/periods
                if normalized_query.contains("quarter") || normalized_query.contains("period") {
                    if let Ok(series) = df.column("quarter") {
                        let quarters: Vec<_> = series.utf8()?.into_iter().flatten().collect();
                        return Ok(format!(
                            "Loaded quarters from SEC: {}",
                            quarters.join(", ")
                        ));
                    }
                }

                // Dynamic metric detection (fuzzy matching, synonyms)
                let available_metrics: Vec<String> = df
                    .get_column_names()
                    .iter()
                    .filter(|&name| name != &"quarter")
                    .map(|s| s.to_lowercase())
                    .collect();

                let synonyms = [
                    ("revenue", "revenues"),
                    ("net income", "netincomeloss"),
                    ("eps", "earningspersharediluted"),
                    ("assets", "assets"),
                    ("liabilities", "liabilities"),
                    ("cash", "cashandcashequivalentsatcarryingvalue"),
                    ("operating cash flow", "operatingcashflow"),
                ];

                let mut found_metric: Option<String> = None;
                for metric in &available_metrics {
                    if normalized_query.contains(metric) {
                        found_metric = Some(metric.clone());
                        break;
                    }
                }
                if found_metric.is_none() {
                    for (syn, canonical) in &synonyms {
                        if normalized_query.contains(syn) && available_metrics.contains(&canonical.to_string()) {
                            found_metric = Some(canonical.to_string());
                            break;
                        }
                    }
                }

                if let Some(metric) = found_metric {
                    let orig_col = df.get_column_names().iter().find(|name| name.to_lowercase() == metric).unwrap();
                    if let Ok(series) = df.column(orig_col) {
                        if let Ok(f64chunked) = series.f64() {
                            let total: f64 = f64chunked.into_iter().flatten().sum();
                            let most_recent = f64chunked.into_iter().flatten().last().unwrap_or(0.0);
                            let avg: f64 = if f64chunked.len() > 0 { total / f64chunked.len() as f64 } else { 0.0 };
                            return Ok(format!(
                                "SEC EDGAR {} Analysis:\n  • Total {} (last {} periods): {:.2}B\n  • Average per period: {:.2}B\n  • Most recent period: {:.2}B",
                                orig_col,
                                orig_col,
                                f64chunked.len(),
                                total,
                                avg,
                                most_recent
                            ));
                        } else if let Ok(utf8chunked) = series.utf8() {
                            let values: Vec<_> = utf8chunked.into_iter().flatten().collect();
                            return Ok(format!(
                                "SEC EDGAR {} values: {}",
                                orig_col,
                                values.join(", ")
                            ));
                        } else {
                            return Ok(format!(
                                "SEC EDGAR: Column '{}' found, but data type is not supported for analysis.",
                                orig_col
                            ));
                        }
                    } else {
                        return Ok(format!(
                            "SEC EDGAR: Metric '{}' detected but could not retrieve data.",
                            orig_col
                        ));
                    }
                }

                let available_list = if available_metrics.is_empty() {
                    "No financial metrics available.".to_string()
                } else {
                    format!("Available metrics: {}", available_metrics.join(", "))
                };
                Ok(format!(
                    "FINFILES AI: Could not detect a specific financial metric in your query.\n{}\nTry asking about one of these metrics, or type 'summarize', 'forecast', 'anomaly', or 'show table'.",
                    available_list
                ))
            }
            fn backend_name(&self) -> &'static str { "FINFILES AI" }
        }

        #[async_trait]
        impl FinancialAIModule for OnnxAIModule {
            async fn analyze(&self, df: &DataFrame, query: &str) -> Result<String> {
                // For this system, ONNX backend is a stub and delegates to FinfilesAI logic.
                FinfilesAI.analyze(df, query).await
            }
            fn backend_name(&self) -> &'static str { "ONNX" }
        }

        #[async_trait]
        impl FinancialAIModule for RemoteLLMAIModule {
            async fn analyze(&self, df: &DataFrame, query: &str) -> Result<String> {
                // For this system, RemoteLLM backend is a stub and delegates to FinfilesAI logic.
                FinfilesAI.analyze(df, query).await
            }
            fn backend_name(&self) -> &'static str { "RemoteLLM" }
        }

        #[async_trait]
        impl FinancialAIModule for CustomModelAIModule {
            async fn analyze(&self, df: &DataFrame, query: &str) -> Result<String> {
                // For this system, custom model backend is a stub and delegates to FinfilesAI logic.
                FinfilesAI.analyze(df, query).await
            }
            fn backend_name(&self) -> &'static str { "CustomModel" }
        }
    }

    pub mod data_ingestion {
        use super::error::*;
        use polars::prelude::*;
        use serde::Deserialize;
        use std::collections::{HashMap, HashSet};
        use reqwest::Client;

        #[derive(Debug, Deserialize)]
        pub struct CikEntry {
            pub cik_str: String,
            pub ticker: String,
            pub title: String,
        }

        #[derive(Debug, Deserialize)]
        pub struct CompanySubmissions {
            pub filings: Filings,
        }

        #[derive(Debug, Deserialize)]
        pub struct Filings {
            pub recent: RecentFilings,
        }

        #[derive(Debug, Deserialize)]
        pub struct RecentFilings {
            pub accession_number: Vec<String>,
            pub form: Vec<String>,
        }

        #[derive(Debug, Deserialize)]
        pub struct CompanyFacts {
            pub facts: HashMap<String, HashMap<String, GaapFact>>,
        }

        #[derive(Debug, Deserialize)]
        pub struct GaapFact {
            pub units: HashMap<String, Vec<FactUnit>>,
        }

        #[derive(Debug, Deserialize)]
        pub struct FactUnit {
            #[serde(rename = "fiscalPeriod")]
            pub fiscal_period: Option<String>,
            #[serde(rename = "val")]
            pub value: Option<f64>,
        }

        pub struct FinancialDataLoader;

        impl FinancialDataLoader {
            // Loads SEC EDGAR data for a user-specified ticker
            pub async fn load_sec_data_for_ticker(ticker: &str) -> Result<DataFrame> {
                log::info!("FINFILES AI: Fetching SEC EDGAR filings for ticker: {}", ticker);

                let client = Client::builder()
                    .timeout(std::time::Duration::from_secs(20))
                    .user_agent("FINFILES AI/1.0 (contact: ai@finfiles.ai)")
                    .build()
                    .map_err(|e| FinAIError::Network(format!("Failed to build HTTP client: {e}")))?;

                // Retry logic for transient network errors
                let mut retries = 0;
                let cik_map: HashMap<String, CikEntry> = loop {
                    match client.get("https://www.sec.gov/files/company_tickers.json").send().await {
                        Ok(resp) => match resp.json().await {
                            Ok(json) => break json,
                            Err(e) => return Err(FinAIError::DataParsing(format!("Failed to parse CIK map: {e}"))),
                        },
                        Err(_e) if retries < 2 => {
                            retries += 1;
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            continue;
                        }
                        Err(e) => return Err(FinAIError::Network(format!("Failed to fetch CIK map: {e}"))),
                    }
                };

                let cik = cik_map.values()
                    .find(|entry| entry.ticker.eq_ignore_ascii_case(ticker))
                    .map(|entry| entry.cik_str.clone())
                    .ok_or_else(|| FinAIError::TickerNotFound(ticker.to_string()))?;

                // Download the most recent 10-K or 10-Q filings (JSON index)
                let filings_url = format!(
                    "https://data.sec.gov/submissions/CIK{:0>10}.json",
                    cik
                );
                let company_submissions: CompanySubmissions = client.get(&filings_url)
                    .send().await
                    .map_err(|e| FinAIError::Network(format!("Failed to fetch company submissions: {e}")))?
                    .json().await
                    .map_err(|e| FinAIError::DataParsing(format!("Failed to parse company submissions: {e}")))?;

                // Find the latest 10-K or 10-Q
                let _idx = company_submissions.filings.recent.form.iter().position(|form| form == "10-K" || form == "10-Q")
                    .ok_or_else(|| FinAIError::SecDataNotFound(ticker.to_string()))?;

                let filing_url = format!(
                    "https://data.sec.gov/api/xbrl/companyfacts/CIK{:0>10}.json",
                    cik
                );

                // Download XBRL company financials
                let facts: CompanyFacts = client.get(&filing_url)
                    .send().await
                    .map_err(|e| FinAIError::Network(format!("Failed to fetch company facts: {e}")))?
                    .json().await
                    .map_err(|e| FinAIError::DataParsing(format!("Failed to parse company facts: {e}")))?;

                // Extract all available metrics for the last 4 quarters
                let mut quarter_set: HashSet<String> = HashSet::new();
                // metric (with currency) -> quarter -> value
                let mut metric_map: HashMap<String, HashMap<String, f64>> = HashMap::new();

                if let Some(us_gaap) = facts.facts.get("us-gaap") {
                    for (metric, fact) in us_gaap {
                        for (currency, units) in &fact.units {
                            for item in units {
                                if let (Some(q), Some(val)) = (item.fiscal_period.as_ref(), item.value) {
                                    quarter_set.insert(q.clone());
                                    let metric_key = format!("{}_{}", metric, currency);
                                    metric_map.entry(metric_key)
                                        .or_default()
                                        .insert(q.clone(), val / 1_000_000_000.0); // billions
                                }
                            }
                        }
                    }
                }

                let mut quarters: Vec<String> = quarter_set.into_iter().collect();
                quarters.sort_by(|a, b| b.cmp(a)); // Descending (most recent first)
                let quarters = quarters.into_iter().take(4).collect::<Vec<_>>();

                if quarters.is_empty() {
                    return Err(FinAIError::SecDataNotFound(ticker.to_string()));
                }

                // Build DataFrame columns dynamically
                let mut columns: Vec<Series> = Vec::new();
                columns.push(Series::new("quarter", &quarters));

                // Include all available financial metrics 
                let preferred_metrics: Vec<&str> = metric_map.keys().map(|k| k.as_str()).collect();
                let mut included_metrics = Vec::new();

                for metric in &preferred_metrics {
                    if let Some(qmap) = metric_map.get(*metric) {
                        let vals: Vec<f64> = quarters.iter().map(|q| qmap.get(q).copied().unwrap_or(0.0)).collect();
                        columns.push(Series::new(metric, vals));
                        included_metrics.push(metric.to_string());
                    }
                }
                // Add any other metrics found 
                for (metric, qmap) in &metric_map {
                    if included_metrics.contains(metric) { continue; }
                    let vals: Vec<f64> = quarters.iter().map(|q| qmap.get(q).copied().unwrap_or(0.0)).collect();
                    columns.push(Series::new(metric, vals));
                }

                let df = DataFrame::new(columns)
                    .map_err(|e| FinAIError::DataParsing(format!("Failed to build DataFrame: {e}")))?;
                Ok(df)
            }
        }
    }

    pub mod chat_ui {
        use super::ai::{FinancialAIModule, CustomModelAIModule};
        use super::error::*;
        use polars::prelude::*;
        use gtk::prelude::*;
        use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, Entry, Orientation, ScrolledWindow, TextView, Spinner, ComboBoxText, FileChooserAction, FileChooserDialog, ResponseType, ListBox, Label, SelectionMode, MessageDialog, MessageType, ButtonsType};
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::path::PathBuf;
        use std::sync::Arc;

        pub struct FinancialAIChatApp {
            ai_modules: Vec<Arc<dyn FinancialAIModule>>,
            data: DataFrame,
            audit_log_path: PathBuf,
            username: String,
        }

        impl FinancialAIChatApp {
            pub fn new(ai_modules: Vec<Arc<dyn FinancialAIModule>>, data: DataFrame, audit_log_path: PathBuf, username: String) -> Self {
                Self { ai_modules, data, audit_log_path, username }
            }

            pub fn run(&self) {
                let app = Application::builder()
                    .application_id("com.finfiles.FINFILES-AI")
                    .build();

                let ai_modules = Rc::new(RefCell::new(self.ai_modules.clone()));
                let data = self.data.clone();
                let audit_log_path = self.audit_log_path.clone();
                let username = self.username.clone();

                app.connect_activate(move |app| {
                    let window = ApplicationWindow::builder()
                        .application(app)
                        .title("FINFILES AI: Financial Data AI Chat")
                        .default_width(1200)
                        .default_height(800)
                        .build();

                    // Accessibility: Set window role and accessible name
                    window.set_accessible_role(gtk::AccessibleRole::Window);
                    window.set_accessible_name(Some("FINFILES AI Main Window"));

                    let vbox = GtkBox::new(Orientation::Vertical, 5);

                    // Chat history
                    let chat_history = TextView::new();
                    chat_history.set_editable(false);
                    chat_history.set_accessible_role(gtk::AccessibleRole::TextBox);
                    chat_history.set_accessible_name(Some("Chat History"));
                    chat_history.set_can_focus(true);

                    let scroll = ScrolledWindow::builder()
                        .child(&chat_history)
                        .min_content_height(400)
                        .build();

                    // User input
                    let user_input = Entry::new();
                    user_input.set_placeholder_text(Some("Ask about SEC data (e.g., 'Show revenue', 'Summarize', 'Forecast', 'Anomaly', 'Show table')"));
                    user_input.set_accessible_name(Some("User Input"));
                    user_input.set_can_focus(true);

                    let send_button = Button::with_label("Send");
                    send_button.set_accessible_name(Some("Send Button"));
                    send_button.set_can_focus(true);

                    // Backend selection
                    let backend_combo = ComboBoxText::new();
                    for module in ai_modules.borrow().iter() {
                        backend_combo.append_text(module.backend_name());
                    }
                    backend_combo.set_active(Some(0));
                    backend_combo.set_accessible_name(Some("Backend Selection"));
                    backend_combo.set_can_focus(true);

                    // Spinner for loading
                    let spinner = Spinner::new();
                    spinner.set_accessible_name(Some("Loading Spinner"));

                    // Save button for exporting DataFrame
                    let save_button = Button::with_label("Save Data");
                    save_button.set_accessible_name(Some("Save Data Button"));
                    save_button.set_can_focus(true);

                    // Upload custom model button
                    let upload_button = Button::with_label("Upload Model");
                    upload_button.set_accessible_name(Some("Upload Model Button"));
                    upload_button.set_can_focus(true);

                    // History panel
                    let history_list = ListBox::new();
                    history_list.set_selection_mode(SelectionMode::None);
                    history_list.set_accessible_name(Some("Chat History List"));
                    history_list.set_can_focus(true);

                    let history_scroll = ScrolledWindow::builder()
                        .child(&history_list)
                        .min_content_width(300)
                        .min_content_height(400)
                        .build();

                    // Layout: left = history, right = chat
                    let hsplit = GtkBox::new(Orientation::Horizontal, 5);
                    hsplit.append(&history_scroll);

                    let chat_vbox = GtkBox::new(Orientation::Vertical, 5);
                    chat_vbox.append(&scroll);

                    let hbox = GtkBox::new(Orientation::Horizontal, 5);
                    hbox.append(&backend_combo);
                    hbox.append(&user_input);
                    hbox.append(&send_button);
                    hbox.append(&spinner);
                    hbox.append(&save_button);
                    hbox.append(&upload_button);

                    chat_vbox.append(&hbox);
                    hsplit.append(&chat_vbox);

                    vbox.append(&hsplit);

                    window.set_child(Some(&vbox));
                    window.show();

                    // Accessibility: Keyboard navigation
                    user_input.grab_focus();

                    // State
                    let chat_history_clone = chat_history.clone();
                    let data_clone = data.clone();
                    let ai_modules = ai_modules.clone();
                    let backend_combo = backend_combo.clone();
                    let user_input = user_input.clone();
                    let spinner = spinner.clone();
                    let history_list = Rc::new(RefCell::new(history_list));
                    let audit_log_path = audit_log_path.clone();
                    let username = username.clone();

                    // Store chat history
                    let chat_history_vec = Rc::new(RefCell::new(Vec::<(String, String, String)>::new())); 

                    // Send button logic
                    let chat_history_vec2 = chat_history_vec.clone();
                    let history_list2 = history_list.clone();
                    send_button.connect_clicked(move |_| {
                        let input_text = user_input.text().to_string();
                        if input_text.trim().is_empty() { return; }
                        spinner.start();

                        // Get selected backend
                        let backend_idx = backend_combo.active().unwrap_or(0) as usize;
                        let ai_modules = ai_modules.borrow();
                        let ai_module = match ai_modules.get(backend_idx) {
                            Some(module) => module.clone(),
                            None => {
                                spinner.stop();
                                let dialog = MessageDialog::new(
                                    Some(&window),
                                    gtk::DialogFlags::MODAL,
                                    MessageType::Error,
                                    ButtonsType::Ok,
                                    "Invalid backend selection.",
                                );
                                dialog.run_async(|d, _| d.close());
                                return;
                            }
                        };
                        let data = data_clone.clone();
                        let chat_history_clone = chat_history_clone.clone();
                        let user_input = user_input.clone();
                        let spinner = spinner.clone();
                        let chat_history_vec = chat_history_vec2.clone();
                        let history_list = history_list2.clone();
                        let audit_log_path = audit_log_path.clone();
                        let username = username.clone();

                        glib::MainContext::default().spawn_local(async move {
                            let response = match ai_module.analyze(&data, &input_text).await {
                                Ok(r) => r,
                                Err(e) => {
                                    log::error!("AI analysis error: {:?}", e);
                                    let dialog = MessageDialog::new(
                                        Some(&chat_history_clone.toplevel().unwrap().downcast::<ApplicationWindow>().unwrap()),
                                        gtk::DialogFlags::MODAL,
                                        MessageType::Error,
                                        ButtonsType::Ok,
                                        &format!("AI analysis error: {e}"),
                                    );
                                    dialog.run_async(|dialog, _| dialog.close());
                                    format!("An error occurred during analysis: {e}")
                                }
                            };
                            if let Some(buffer) = chat_history_clone.buffer() {
                                buffer.insert_at_cursor(&format!("User ({}): {}\nFINFILES AI: {}\n", ai_module.backend_name(), input_text, response));
                            }
                            user_input.set_text("");

                            // Add to history panel
                            let row = gtk::ListBoxRow::new();
                            let label = Label::new(Some(&format!("{}: {}", ai_module.backend_name(), input_text)));
                            row.set_child(Some(&label));
                            history_list.borrow().append(&row);

                            // Store in chat history vector
                            chat_history_vec.borrow_mut().push((ai_module.backend_name().to_string(), input_text.clone(), response.clone()));

                            // Audit log
                            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&audit_log_path) {
                                let _ = writeln!(file, "[{}][user:{}] User: {}\nAI: {}\n", ai_module.backend_name(), username, input_text, response);
                            }

                            spinner.stop();
                        });
                    });

                    // Save button logic
                    let data_for_save = data.clone();
                    save_button.connect_clicked(move |_| {
                        let dialog = FileChooserDialog::new(
                            Some("Save Data As"),
                            Some(&window),
                            FileChooserAction::Save,
                            &[("Cancel", ResponseType::Cancel), ("Save", ResponseType::Accept)],
                        );
                        dialog.set_current_name("finfiles_ai_data.csv");
                        dialog.run_async(move |dialog, resp| {
                            if resp == ResponseType::Accept {
                                if let Some(path) = dialog.file().and_then(|f| f.path()) {
                                    if let Err(e) = data_for_save.write_csv(&path) {
                                        let err_dialog = MessageDialog::new(
                                            Some(&window),
                                            gtk::DialogFlags::MODAL,
                                            MessageType::Error,
                                            ButtonsType::Ok,
                                            &format!("Failed to save CSV: {e}"),
                                        );
                                        err_dialog.run_async(|d, _| d.close());
                                    }
                                }
                            }
                            dialog.close();
                        });
                    });

                    // Upload custom model logic
                    let ai_modules_upload = ai_modules.clone();
                    let backend_combo_upload = backend_combo.clone();
                    upload_button.connect_clicked(move |_| {
                        let dialog = FileChooserDialog::new(
                            Some("Upload Model"),
                            Some(&window),
                            FileChooserAction::Open,
                            &[("Cancel", ResponseType::Cancel), ("Upload", ResponseType::Accept)],
                        );
                        dialog.run_async(move |dialog, resp| {
                            if resp == ResponseType::Accept {
                                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                                    // For demo: just use file name as model name
                                    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("CustomModel").to_string();
                                    if let Ok(custom_module) = CustomModelAIModule::new(name.clone()) {
                                        ai_modules_upload.borrow_mut().push(Arc::new(custom_module));
                                        backend_combo_upload.append_text("CustomModel");
                                    }
                                }
                            }
                            dialog.close();
                        });
                    });

                    // Accessibility: Keyboard shortcuts (Enter to send, Ctrl+S to save, Ctrl+U to upload)
                    let send_button_shortcut = send_button.clone();
                    let save_button_shortcut = save_button.clone();
                    let upload_button_shortcut = upload_button.clone();
                    window.add_controller(&gtk::EventControllerKey::new().connect_key_pressed(move |_, key, _, _| {
                        match key.keyval() {
                            gdk::keys::constants::Return => {
                                if user_input.has_focus() {
                                    send_button_shortcut.emit_clicked();
                                    return true;
                                }
                            }
                            gdk::keys::constants::S if key.state().contains(gdk::ModifierType::CONTROL_MASK) => {
                                save_button_shortcut.emit_clicked();
                                return true;
                            }
                            gdk::keys::constants::U if key.state().contains(gdk::ModifierType::CONTROL_MASK) => {
                                upload_button_shortcut.emit_clicked();
                                return true;
                            }
                            _ => {}
                        }
                        false
                    }));

                });

        window.add(&vbox);
        window.show_all();
        window
    }

    use std::sync::Arc;
    use polars::prelude::*;
    use crate::ai::{FinfilesAI, OnnxAIModule, RemoteLLMAIModule, FinancialAIModule, CustomModelAIModule};
    use crate::data_ingestion::FinancialDataLoader;
    use crate::chat_ui::FinancialAIChatApp;
    use crate::error::*;

    // Main Entry Point - FINFILES AI

    #[tokio::main]
    async fn main() -> Result<()> {
        env_logger::init();
        log::info!("Starting AA SEC EDGAR + FINFILES AI: Unified Financial Data & AI Platform...");

        // Prompt user for ticker (for initial SEC data and AI chat)
        println!("Enter the stock ticker symbol (e.g., AAPL, MSFT): ");
        let mut ticker = String::new();
        std::io::stdin().read_line(&mut ticker)?;
        let ticker = ticker.trim();

        // Data ingestion from SEC EDGAR (async, with loading indicator in UI)
        println!("Loading SEC EDGAR data for {ticker}...");
        let ai_data = match FinancialDataLoader::load_sec_data_for_ticker(ticker).await {
            Ok(df) => Some(df),
            Err(e) => {
                eprintln!("Error loading SEC data: {e}");
                None
            }
        };

        // Modular AI/ML engine selection (EDGAR-powered, ready for multi-backend)
        let ai_modules: Vec<Arc<dyn FinancialAIModule>> = vec![
            Arc::new(FinfilesAI::new()?),
            Arc::new(OnnxAIModule::new()?),
            Arc::new(RemoteLLMAIModule::new()?),
            Arc::new(CustomModelAIModule::new()?),
            // CustomModelAIModule(s) can be added at runtime via UI
        ];

        // Security, backend, and GTK app setup
        logging::init();
        security::init_tls();
        let auth = Arc::new(AuthManager::new());

        // Authenticate user (OAuth2, OIDC, etc.)
        let user = auth.authenticate_user();
        if user.is_none() {
            eprintln!("Authentication failed. Exiting.");
            return Ok(());
        }
        let user = user.unwrap();

        // RBAC: Only allow access to permitted tickers/features
        if !auth.has_role(&user, RBACRole::User) {
            eprintln!("Access denied. Contact support.");
            return Ok(());
        }

        // Start backend microservices (API, DB, cache, websocket, analytics)
        backend::start_services();

        // GTK Application: Unified SEC EDGAR,FINFILES AI UI
        let app = Application::new(
            Some("com.aa.sec_edgar_finfiles_ai"),
            Default::default(),
        );

        let state = Arc::new(AppState::new(user.clone()));
        let auth_arc = auth.clone();
        let ai_modules_for_ui = ai_modules.clone();
        let ai_data_for_ui = ai_data.clone();
        let audit_log_path_for_ui = audit_log_path.clone();
        let username_for_ui = username.clone();

        app.connect_activate(move |app| {
            let window = build_main_window(
                app,
                state.clone(),
                auth_arc.clone(),
                ai_modules_for_ui.clone(),
                ai_data_for_ui.clone(),
                audit_log_path_for_ui.clone(),
                username_for_ui.clone(),
            );
            window.present();
        });

        app.run();

        Ok(())
    }
