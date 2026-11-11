import os
import sys
import argparse
from pathlib import Path
from sqlalchemy.orm import Session

# Add the backend directory to sys.path so we can import app modules
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from app.db import SessionLocal
from app.services.import_workflow_service import ImportWorkflowService

def import_directory_direct(directory_path: str, confirm: bool = False):
    path = Path(directory_path)
    if not path.exists():
        print(f"Error: Directory {directory_path} does not exist.")
        sys.exit(1)

    print(f"Scanning directory: {directory_path}")
    
    csv_files = []
    for root, dirs, files in os.walk(path):
        for file in files:
            if file.endswith('.csv') and 'SolieuGD' in root:
                csv_files.append(os.path.join(root, file))
                
    if not csv_files:
        print("No valid SolieuGD CSV files found.")
        return

    print(f"Found {len(csv_files)} files. Starting safe workflow import...\n")
    
    db: Session = SessionLocal()
    try:
        total_imported = 0
        total_skipped = 0
        
        for file_path in csv_files:
            filename = os.path.basename(file_path)
            print(f"Processing {filename}...")
            try:
                with open(file_path, 'rb') as f:
                    file_content = f.read()
                    
                preview = ImportWorkflowService.preview_import(
                    db=db,
                    file_content=file_content,
                    filename=filename,
                    source_type="cafef",
                    timeframe="1D",
                    adjustment_type="unadjusted"
                )
                print(f"  Preview: run_id='{preview.run_id}', can_accept={preview.can_accept}, parsed={preview.parsed_count}")

                if not confirm:
                    print(f"  [XÁC NHẬN BỊ BỎ QUA]: Không chấp nhận lượt nhập '{preview.run_id}' khi chưa có cờ --confirm.")
                    total_skipped += preview.parsed_count
                    continue

                if not preview.can_accept:
                    print(f"  Blocked {filename}: {preview.block_reason}")
                    total_skipped += preview.rejected_count
                    continue

                accept_res = ImportWorkflowService.accept_import(
                    db=db,
                    run_id=preview.run_id,
                    content_sha256=preview.content_sha256
                )
                print(f"  Success: Accepted {accept_res.accepted_count} rows, Skipped {preview.rejected_count} rows.")
                total_imported += accept_res.accepted_count
                total_skipped += preview.rejected_count
            except Exception as e:
                print(f"  Error processing {file_path}: {e}")
                db.rollback()
                
        if not confirm:
            print(f"\n[CHẾ ĐỘ XEM TRƯỚC HOÀN TẤT] Không có dữ liệu nến/mã chứng khoán nào bị thay đổi. Chạy lại với '--confirm' để chấp nhận chính thức.")
        else:
            print(f"\nImport Complete! Total Imported: {total_imported}, Total Skipped: {total_skipped}")
    finally:
        db.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Batch Import CafeF SolieuGD data directly to DB.")
    parser.add_argument("directory", help="Path to the CafeFData directory")
    parser.add_argument("--confirm", action="store_true", help="Confirm acceptance of previewed import runs")
    
    args = parser.parse_args()
    import_directory_direct(args.directory, confirm=args.confirm)
