import os
import glob
from app.db import SessionLocal, Base, engine
from app.services.cafef_importer import CafeFImporter

def seed_cafef_data():
    Base.metadata.create_all(bind=engine)
    db = SessionLocal()
    
    data_dir = r"E:\Workspace\sumi\data\raw\cafef_sample"
    # We only process the large files from the root of data\raw\cafef_sample
    files = glob.glob(os.path.join(data_dir, "*.csv"))
    
    # We ignore NN_ files (Foreign ownership) for now, as our system OHLCV doesn't map NN_ easily
    target_files = [f for f in files if "NN_" not in os.path.basename(f)]
    
    print(f"Found {len(target_files)} data files to import.")
    
    for file_path in target_files:
        filename = os.path.basename(file_path)
        adjustment_type = 'adjusted' if 'CC_' in filename else 'unadjusted'
        print(f"Importing {filename} as {adjustment_type}...")
        
        try:
            with open(file_path, "rb") as f:
                content = f.read()
                
            res = CafeFImporter.import_data(db, content, filename, adjustment_type=adjustment_type)
            print(f"SUCCESS {filename}: Imported {res.imported_rows}, Symbols {res.symbols_count}, Dates {res.start_date} to {res.end_date}")
        except Exception as e:
            print(f"FAILED {filename}: {str(e)}")
            
    db.close()
    print("CafeF Seed complete!")

if __name__ == "__main__":
    seed_cafef_data()
