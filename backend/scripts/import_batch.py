import os
import sys
import argparse
from pathlib import Path
from sqlalchemy.orm import Session

# Add the backend directory to sys.path so we can import app modules
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from app.db import SessionLocal
from app.services.cafef_importer import CafeFImporter

def import_directory_direct(directory_path: str):
    path = Path(directory_path)
    if not path.exists():
        print(f"Error: Directory {directory_path} does not exist.")
        sys.exit(1)

    print(f"Scanning directory: {directory_path}")
    
    csv_files = []
    # Walk through the directory to find CafeF.SolieuGD directories
    for root, dirs, files in os.walk(path):
        for file in files:
            if file.endswith('.csv') and 'SolieuGD' in root:
                csv_files.append(os.path.join(root, file))
                
    if not csv_files:
        print("No valid SolieuGD CSV files found.")
        return

    print(f"Found {len(csv_files)} files. Starting direct database import...\n")
    
    db: Session = SessionLocal()
    try:
        total_imported = 0
        total_skipped = 0
        
        for file_path in csv_files:
            print(f"Importing {os.path.basename(file_path)}...")
            try:
                with open(file_path, 'rb') as f:
                    file_content = f.read()
                    result = CafeFImporter.import_data(db, file_content, os.path.basename(file_path))
                    
                print(f"  Success: Imported {result.imported_rows} rows, Skipped {result.skipped_rows} rows.")
                total_imported += result.imported_rows
                total_skipped += result.skipped_rows
            except Exception as e:
                print(f"  Error processing {file_path}: {e}")
                db.rollback()
                
        print(f"\nImport Complete! Total Imported: {total_imported}, Total Skipped: {total_skipped}")
    finally:
        db.close()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Batch Import CafeF SolieuGD data directly to DB.")
    parser.add_argument("directory", help="Path to the CafeFData directory")
    
    args = parser.parse_args()
    import_directory_direct(args.directory)
