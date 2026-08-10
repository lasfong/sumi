import os
import glob
from app.db import SessionLocal, Base, engine
from app.services.import_workflow_service import ImportWorkflowService

import argparse

def seed_cafef_data(confirm: bool = False):
    Base.metadata.create_all(bind=engine)
    db = SessionLocal()
    
    data_dir = r"E:\Workspace\sumi\data\raw\cafef_sample"
    files = glob.glob(os.path.join(data_dir, "*.csv"))
    target_files = [f for f in files if "NN_" not in os.path.basename(f)]
    
    print(f"Found {len(target_files)} data files to preview/import.")
    
    for file_path in target_files:
        filename = os.path.basename(file_path)
        adjustment_type = 'adjusted' if 'CC_' in filename else 'unadjusted'
        print(f"Processing {filename} as {adjustment_type}...")
        
        try:
            with open(file_path, "rb") as f:
                content = f.read()
                
            preview = ImportWorkflowService.preview_import(
                db=db,
                file_content=content,
                filename=filename,
                source_type="cafef",
                timeframe="1D",
                adjustment_type=adjustment_type
            )
            print(f"  PREVIEW {filename}: run_id='{preview.run_id}', can_accept={preview.can_accept}, parsed={preview.parsed_count}")

            if not confirm:
                print(f"  [XÁC NHẬN BỊ BỎ QUA]: Không chấp nhận lượt nhập '{preview.run_id}' khi chưa có cờ --confirm.")
                continue

            if not preview.can_accept:
                print(f"  BLOCKED {filename}: {preview.block_reason}")
                continue

            accept_res = ImportWorkflowService.accept_import(
                db=db,
                run_id=preview.run_id,
                content_sha256=preview.content_sha256
            )
            print(f"  ACCEPTED {filename}: Accepted {accept_res.accepted_count} rows (status: {accept_res.status})")
        except Exception as e:
            print(f"  FAILED {filename}: {str(e)}")
            
    db.close()
    if not confirm:
        print("\n[CHẾ ĐỘ XEM TRƯỚC HOÀN TẤT] Không có dữ liệu nến/mã chứng khoán nào bị thay đổi. Chạy lại với '--confirm' để chấp nhận chính thức.")
    else:
        print("\nCafeF Seed complete!")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Seed CafeF sample data with safe workflow.")
    parser.add_argument("--confirm", action="store_true", help="Confirm acceptance of previewed import runs")
    args = parser.parse_args()
    seed_cafef_data(confirm=args.confirm)
