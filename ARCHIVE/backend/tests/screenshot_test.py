"""Manual screenshot helper; intentionally excluded from normal pytest collection."""


def capture_screenshot() -> None:
    from playwright.sync_api import sync_playwright

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        page = browser.new_page()
        page.goto("http://localhost:5173/")
        page.wait_for_timeout(3000)
        page.screenshot(path="screenshot.png")
        print("Screenshot saved to screenshot.png")
        browser.close()


if __name__ == "__main__":
    capture_screenshot()
