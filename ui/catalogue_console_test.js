// PASTE THIS INTO YOUR BROWSER CONSOLE TO TEST THE CATALOGUE TAB

// First, let's check what exists
console.log('=== CATALOGUE TAB DIAGNOSTIC ===');

// Check if the button exists
const catalogueButton = document.querySelector('button[onclick*="catalogue"]');
console.log('Catalogue button found:', !!catalogueButton);

// Check if the tab content exists
const catalogueTab = document.getElementById('catalogueTab');
console.log('Catalogue tab content found:', !!catalogueTab);

// Check if switchTab function exists
console.log('switchTab function exists:', typeof switchTab !== 'undefined');

// Override the switchTab function with a working version
window.switchTab = function(tabName, element) {
    console.log('Manual switchTab called for:', tabName);
    
    // Update buttons
    document.querySelectorAll('.sidebar-tab').forEach(tab => {
        tab.classList.remove('active');
    });
    if (element) element.classList.add('active');
    
    // Update content
    document.querySelectorAll('.tab-content').forEach(content => {
        content.classList.remove('active');
    });
    
    const target = document.getElementById(tabName + 'Tab');
    if (target) {
        target.classList.add('active');
        console.log('Successfully switched to:', tabName);
        
        // Load catalogue
        if (tabName === 'catalogue') {
            const grid = document.getElementById('strategyGrid');
            if (grid) {
                grid.innerHTML = `
                    <div style="padding: 20px; background: #f0f0f0; border-radius: 8px; margin: 10px;">
                        <h3>✅ Catalogue Tab is Working!</h3>
                        <p>The tab switching functionality is now active.</p>
                        <p>Strategy cards would be loaded here.</p>
                    </div>
                `;
            }
        }
    } else {
        console.error('Tab not found:', tabName + 'Tab');
    }
};

// Try to click the catalogue tab
if (catalogueButton) {
    console.log('Attempting to click Catalogue tab...');
    catalogueButton.click();
} else {
    console.error('Cannot find Catalogue button to click');
}

console.log('=== END DIAGNOSTIC ===');
console.log('If you see "Catalogue Tab is Working!" above, the fix worked.');
console.log('If not, check for JavaScript errors in the console.');