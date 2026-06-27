import re
import pytest
from playwright.sync_api import Page, expect

def test_session_setup_and_replay(page: Page):
    # TC-01: Khởi tạo Phiên Giao dịch
    page.goto("http://localhost:5173/replay")
    page.wait_for_timeout(2000)

    # Fill in symbol
    symbol_input = page.locator('input[placeholder*="Search symbol"]')
    symbol_input.fill("FPT")
    page.wait_for_timeout(1000)
    page.get_by_text("FPT", exact=True).first.click()
    
    # Fill initial cash
    cash_input = page.get_by_text("Initial Cash").locator("..").locator("input")
    cash_input.fill("100000000")
    
    # Click Start Replay
    page.get_by_role("button", name="▶ Start Replay").click()
    
    # Expect to navigate to replay page and see symbol
    page.wait_for_timeout(5000)
    page.screenshot(path="screenshot.png")
    expect(page.get_by_text("FPT", exact=True).first).to_be_visible(timeout=10000)
    
    # TC-02: Kiểm tra Dữ liệu Stream
    # Click Auto-Play
    auto_play_btn = page.get_by_role("button", name=re.compile("Auto-Play", re.IGNORECASE))
    expect(auto_play_btn).to_be_visible()
    auto_play_btn.click()
    
    # Wait for a few candles to stream
    page.wait_for_timeout(3000)
    
    # Click Pause
    pause_btn = page.get_by_role("button", name=re.compile("Pause", re.IGNORECASE))
    expect(pause_btn).to_be_visible()
    pause_btn.click()
    
    # TC-03: Kiểm thử Ràng buộc T+2
    # Click BUY button on sidebar
    page.get_by_role("button", name="BUY").click()
    page.wait_for_timeout(500)
    
    # Fill in quantity in modal
    qty_input = page.locator("input[type='number']").first
    qty_input.fill("100")
    
    # Submit the BUY order
    page.get_by_role("button", name="Submit").click()
    page.wait_for_timeout(500)
    
    # Try to sell immediately
    page.get_by_role("button", name="SELL").click()
    page.wait_for_timeout(500)
    page.get_by_role("button", name="Submit").click()
    
    # Should see toast error
    expect(page.get_by_text(re.compile("Cannot sell: T\\+2 constraint"))).to_be_visible(timeout=5000)
    
    # Close the toast or modal if it's still open
    # We can just click Cancel if the modal is open
    cancel_btn = page.get_by_role("button", name="Cancel")
    if cancel_btn.is_visible():
        cancel_btn.click()
    
    # TC-04: Tua qua T+2 và Bán
    # Next 2 candles
    next_btn = page.get_by_role("button", name="Next →")
    next_btn.click()
    page.wait_for_timeout(1000)
    next_btn.click()
    page.wait_for_timeout(1000)
    
    # Sell now
    page.get_by_role("button", name="SELL").click()
    page.wait_for_timeout(500)
    page.get_by_role("button", name="Submit").click()
    
    # We should not see the T+2 error, instead the order should go through.
    page.wait_for_timeout(1000)
    expect(page.get_by_text(re.compile("Cannot sell: T\\+2 constraint"))).not_to_be_visible()
