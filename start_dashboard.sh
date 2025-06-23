#!/bin/bash
# Start Fixed Ratio Trading Dashboard Web Server
# Serves the dashboard on http://localhost:3000

echo "🌐 Starting Fixed Ratio Trading Dashboard"
echo "========================================"

# Check if Python 3 is available
if command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
elif command -v python &> /dev/null; then
    PYTHON_CMD="python"
else
    echo "❌ Python not found. Please install Python to run the web server."
    exit 1
fi

# Check if dashboard directory exists
if [ ! -d "dashboard" ]; then
    echo "❌ Dashboard directory not found. Make sure you're in the project root."
    exit 1
fi

# Check if dashboard files exist
if [ ! -f "dashboard/index.html" ] || [ ! -f "dashboard/dashboard.js" ]; then
    echo "❌ Dashboard files not found. Please run the deployment script first."
    exit 1
fi

# Start the web server
echo "📊 Starting web server on http://localhost:3000"
echo "🔗 Dashboard URL: http://localhost:3000"
echo ""
echo "📝 Make sure your local Solana validator is running!"
echo "   If not, run: ./deploy_local.sh"
echo ""
echo "🛑 Press Ctrl+C to stop the server"
echo ""

cd dashboard
$PYTHON_CMD -m http.server 3000

echo ""
echo "🛑 Dashboard server stopped" 