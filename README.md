# DiunDash 🐳

A modern, dark-themed dashboard for visualizing and managing Diun (Docker Image Update Notifier) webhook data. Built with Rust and Actix-web for high performance and reliability.

## Features

- 🎨 **Modern Dark UI** - Beautiful dark theme optimized for monitoring dashboards
- 📊 **Grid View** - Visual grid display of all container images with essential information
- 🔍 **Hover Details** - Detailed information appears on hover for each image
- ⚠️ **Age Indicators** - Color-coded cards based on image age:
  - Normal (within 1 week)
  - Yellow (over 1 week old)
  - Red (over 1 month old)
- 🗑️ **Admin Panel** - Delete images directly from the admin interface (`/admin`)
- 🔗 **Quick Links** - Click any image card to open its Docker Hub page
- 📅 **Smart Dates** - Relative time display (e.g., "2h ago", "3d ago")
- 🔄 **Auto-refresh** - Automatically updates every 30 seconds

## Screenshots

The dashboard displays container images in a responsive grid layout with:
- Image name at the top
- Whale icon in the center
- Full image path
- Last updated timestamp
- Age-based color coding

## Installation

### Using Docker (Recommended)

```bash
# Build the image
docker build -t diundash .

# Run the container
docker run -d \
  -p 5030:5030 \
  -v $(pwd)/data:/app/data \
  --name diundash \
  diundash
```

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd DiunWeb

# Build the project
cargo build --release

# Run the application
./target/release/diundash
```

## Configuration

### Diun Webhook Setup

Configure Diun to send webhooks to DiunDash:

```yaml
# diun.yml
notif:
  webhook:
    endpoint: http://your-server:5030/api/diun
    method: POST
    headers:
      Content-Type: application/json
```

## Usage

### Main Dashboard

- **URL**: `http://localhost:5030/`
- View all container images in a grid layout
- Click any card to open the Docker Hub page
- Hover over cards to see detailed information
- Fixed admin button (⚙️) in the bottom-right corner

### Admin Panel

- **URL**: `http://localhost:5030/admin`
- Same view as main dashboard with delete functionality
- Click the ✕ button on any card to delete it from the database
- Confirmation dialog prevents accidental deletions

## API Endpoints

- `GET /api/webhooks` - Get all container images
- `POST /api/diun` - Receive Diun webhook (used by Diun)
- `DELETE /api/webhooks/{image}` - Delete an image (admin only)

## Data Storage

DiunDash uses SQLite to store webhook data. The database is located at:
- Local: `./data/diun.db`
- Docker: `/app/data/diun.db` (mounted volume)

## Development

### Prerequisites

- Rust 1.75 or later
- Cargo

### Building

```bash
cargo build --release
```

### Running in Development

```bash
cargo run
```

## Technology Stack

- **Backend**: Rust with Actix-web
- **Database**: SQLite (via rusqlite)
- **Frontend**: Vanilla JavaScript with modern CSS
- **Containerization**: Docker with multi-stage builds

## Project Structure

```
DiunWeb/
├── src/
│   └── main.rs          # Main application code
├── static/
│   └── index.html       # Frontend dashboard
├── data/                # SQLite database (created at runtime)
├── Cargo.toml           # Rust dependencies
├── Dockerfile           # Docker build configuration
└── README.md            # This file
```

## License

[Add your license here]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Built for use with [Diun](https://github.com/crazy-max/diun) - Docker Image Update Notifier
- Inspired by the need for better visualization of container image updates

---

Made with ❤️ and Rust

