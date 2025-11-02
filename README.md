# DataSpeak

<div align="center">

**The AI-Powered Database Client**

[![Download for macOS](https://img.shields.io/badge/Download-macOS-blue?style=for-the-badge&logo=apple)](https://dataspeak.co)
[![Download for Windows](https://img.shields.io/badge/Download-Windows-blue?style=for-the-badge&logo=windows)](https://dataspeak.co)
[![Download for Linux](https://img.shields.io/badge/Download-Linux-blue?style=for-the-badge&logo=linux)](https://dataspeak.co)

[Website](https://dataspeak.co) • [Documentation](#getting-started) • [Report Bug](https://github.com/TrevorIkky/DataSpeak/issues) • [Request Feature](https://github.com/TrevorIkky/DataSpeak/issues)

</div>

---

## What is DataSpeak?

DataSpeak is a modern, AI-powered database management application that revolutionizes how you interact with your data. Ask questions in plain English, and DataSpeak translates them into SQL queries, executes them, and presents results with intelligent visualizations—all while giving you the power of a traditional database client.

Built with **Rust** and **React**, DataSpeak combines native performance with cutting-edge AI to make database analysis accessible to everyone, from seasoned developers to business analysts.

---

## ✨ Key Features

<table>
<thead>
<tr>
<th width="40%">Feature</th>
<th width="60%">Description</th>
</tr>
</thead>
<tbody>
<tr>
<td>🤖 <strong>AI-Powered Natural Language Queries</strong></td>
<td>Ask questions in plain English and let DataSpeak handle the SQL. Powered by a sophisticated ReAct agent with native LLM tool calling, DataSpeak understands your database schema and generates accurate queries automatically. Simply ask <em>"Show me the top 5 customers by revenue this year"</em> and get instant results with visualizations.</td>
</tr>
<tr>
<td>📊 <strong>Intelligent Visualizations</strong></td>
<td>DataSpeak automatically detects the best way to visualize your data with <strong>7 chart types</strong> (Bar, Line, Area, Pie, Scatter, Radar, Radial). The AI recognizes temporal and numeric columns, chooses the right chart type, and displays data and visualizations side-by-side.</td>
</tr>
<tr>
<td>🗺️ <strong>Geographic Data Support</strong></td>
<td>First-class support for PostGIS and spatial data. Click any geometry column to view it on an interactive map powered by Leaflet. Parse and display Well-Known Text (WKT) geometries with support for points, lines, polygons, and complex geometries.</td>
</tr>
<tr>
<td>💾 <strong>Multi-Database Support</strong></td>
<td>Connect to PostgreSQL (with PostGIS), MySQL, and MariaDB from a single, unified interface. Credentials are encrypted and stored securely using Tauri Stronghold with Argon2id password hashing.</td>
</tr>
<tr>
<td>⚡ <strong>Smart SQL Editor</strong></td>
<td>Write SQL faster with context-aware autocomplete that understands your database schema. Get suggestions for SQL keywords, table names (with row counts), column names (with data types and key indicators), and SQL functions.</td>
</tr>
<tr>
<td>🔧 <strong>Editable Data Grids</strong></td>
<td>Edit query results directly and commit changes back to your database. Features click-to-edit cells with auto-save, insert/update/delete rows, change tracking before commit, and full CRUD operation support.</td>
</tr>
<tr>
<td>📂 <strong>Import/Export Made Easy</strong></td>
<td>Move data in and out with powerful tools supporting CSV and compressed ZIP archives. Includes multi-threaded parallel processing, real-time progress tracking, and cancellable long-running operations.</td>
</tr>
<tr>
<td>🔍 <strong>Schema Browser & ERD</strong></td>
<td>Explore your database structure visually with a collapsible tree view of all tables and columns. View column details, keys, and relationships, plus auto-generated Entity Relationship Diagrams (ERD) with visual representation of foreign key relationships. You can download ERD diagrams for documentation and sharing.</td>
</tr>
<tr>
<td>🔐 <strong>Security First</strong></td>
<td>Your data security is paramount with encrypted credential storage using Stronghold, production-grade Argon2id password hashing, read-only AI queries (only SELECT statements), and SQL injection prevention with parameterized queries.</td>
</tr>
</tbody>
</table>

---

## 📥 Installation

DataSpeak is available for **macOS**, **Windows**, and **Linux**. Download the latest version for your platform:

### **macOS**

1. Visit [dataspeak.co](https://dataspeak.co)
2. Download the `.dmg` file for macOS
3. Open the downloaded file and drag DataSpeak to your Applications folder
4. Launch DataSpeak from Applications

**System Requirements:**
- macOS 10.15 (Catalina) or later
- Apple Silicon (M1/M2/M3) or Intel processor

### **Windows**

1. Visit [dataspeak.co](https://dataspeak.co)
2. Download the `.msi` installer for Windows
3. Run the installer and follow the setup wizard
4. Launch DataSpeak from the Start menu

**System Requirements:**
- Windows 10 (64-bit) or later
- 4GB RAM minimum

### **Linux**

1. Visit [dataspeak.co](https://dataspeak.co)
2. Download the appropriate package for your distribution:
   - `.deb` for Debian/Ubuntu
   - `.rpm` for Fedora/RHEL
   - `.AppImage` for universal compatibility

**Debian/Ubuntu:**
```bash
sudo dpkg -i dataspeak_*.deb
```

**Fedora/RHEL:**
```bash
sudo rpm -i dataspeak_*.rpm
```

**AppImage:**
```bash
chmod +x dataspeak_*.AppImage
./dataspeak_*.AppImage
```

**System Requirements:**
- glibc 2.31 or later
- GTK 3.24 or later (for system dialogs)
- 4GB RAM minimum

---

## 🚀 Getting Started

### 1. **Connect to Your Database**

Launch DataSpeak and click **"Add Connection"** to create your first database connection:

1. Enter a friendly name for your connection
2. Select your database type (PostgreSQL, MySQL, or MariaDB)
3. Enter your database credentials:
   - Host and port
   - Database name
   - Username and password
4. Click **"Test Connection"** to verify
5. Click **"Save"** to store your connection securely

Your credentials are encrypted and stored locally using Tauri Stronghold—they never leave your machine.

### 2. **Browse Your Schema**

Once connected, explore your database structure in the **Schema Browser**:

- Expand tables to view columns, data types, and keys
- Click any table to view its contents
- See row counts at a glance
- Right-click for quick actions

### 3. **Ask Questions in Natural Language**

Open the **AI Chat** tab and start asking questions:

- "What are the top 10 products by sales?"
- "Show me user registrations over time"
- "Which categories have the most items?"
- "Display customers on a map"

DataSpeak generates the SQL, executes it, and presents results with intelligent visualizations.

### 4. **Write SQL Queries**

Prefer to write SQL? Use the **Query Editor** with smart autocomplete:

1. Open the Query Editor
2. Start typing your query
3. Use autocomplete suggestions (Ctrl/Cmd + Space)
4. Execute with Ctrl/Cmd + Enter

### 5. **Edit Data Directly**

Make changes to your data with editable grids:

1. Run a query to display results
2. Click any cell to edit
3. Changes are tracked automatically
4. Click **"Commit Changes"** to save to the database

### 6. **Export Your Data**

Export tables or query results:

1. Navigate to **Export/Import**
2. Select tables to export
3. Choose CSV or ZIP format
4. Click **"Export"** and select destination

---

## 🎯 Use Cases

DataSpeak is perfect for:

- **Data Analysts**: Query databases without memorizing SQL syntax
- **Business Intelligence**: Generate insights and visualizations on the fly
- **Developers**: Rapid database exploration with smart autocomplete
- **Data Scientists**: Quick data extraction and analysis
- **Product Managers**: Self-service data access without SQL knowledge
- **Geographic Analysis**: Visualize spatial data with built-in mapping
- **Database Administrators**: Multi-database management from one tool

---

## 🤝 Contributing

DataSpeak is open source and welcomes contributions! Whether you're fixing bugs, adding features, or improving documentation, we'd love your help.

### **Getting Started with Development**

1. **Clone the repository:**
   ```bash
   git clone https://github.com/TrevorIkky/DataSpeak.git
   cd DataSpeak
   ```

2. **Install dependencies:**
   ```bash
   pnpm install
   ```

3. **Set up Rust environment:**
   ```bash
   rustup update
   cd src-tauri
   cargo build
   ```

4. **Run in development mode:**
   ```bash
   pnpm tauri dev
   ```

### **Project Structure**

```
dataspeak/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── stores/            # Zustand state management
│   └── types/             # TypeScript definitions
├── src-tauri/             # Rust backend
│   └── src/
│       ├── ai/            # AI agent and OpenRouter integration
│       ├── db/            # Database connection and queries
│       ├── import_export/ # Import/export functionality
│       └── storage/       # Stronghold encrypted storage
└── README.md
```

### **Contribution Guidelines**

- Fork the repository and create a feature branch
- Write clear commit messages
- Add tests for new features
- Update documentation as needed
- Submit a pull request with a detailed description

---

## 📜 License

DataSpeak is released under the [MIT License](https://github.com/TrevorIkky/DataSpeak/blob/main/LICENSE). Feel free to use, modify, and distribute it as you see fit.

---

## 🌟 Why DataSpeak?

In a world where data drives decisions, accessing and understanding that data shouldn't require advanced SQL knowledge. DataSpeak bridges the gap between powerful database tools and natural human language, making data analysis accessible to everyone while giving experts the advanced features they need.

**Traditional database clients** require you to speak SQL.
**DataSpeak** speaks your language.

---

## 🔗 Links

- **Website**: [dataspeak.co](https://dataspeak.co)
- **GitHub**: [github.com/TrevorIkky/DataSpeak](https://github.com/TrevorIkky/DataSpeak)
- **Issues**: [Report bugs or request features](https://github.com/TrevorIkky/DataSpeak/issues)

---

<div align="center">

Made with ❤️ by the DataSpeak team

**[Download Now](https://dataspeak.co)** • **[Star on GitHub](https://github.com/TrevorIkky/DataSpeak)**

</div>
