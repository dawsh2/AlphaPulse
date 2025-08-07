// PASTE THIS IN BROWSER CONSOLE TO FIX TABS

console.log('=== TAB DIAGNOSTIC ===');

// Check if elements exist
const snippetsBtn = document.querySelector('button[onclick*="snippets"]');
const templatesBtn = document.querySelector('button[onclick*="templates"]');
const notebooksBtn = document.querySelector('button[onclick*="notebooks"]');
const catalogueBtn = document.querySelector('button[onclick*="catalogue"]');

console.log('Buttons found:');
console.log('- Snippets:', !!snippetsBtn);
console.log('- Templates:', !!templatesBtn); 
console.log('- Notebooks:', !!notebooksBtn);
console.log('- Catalogue:', !!catalogueBtn);

// Check if tab content exists
console.log('\nTab content found:');
console.log('- snippetsTab:', !!document.getElementById('snippetsTab'));
console.log('- templatesTab:', !!document.getElementById('templatesTab'));
console.log('- notebooksTab:', !!document.getElementById('notebooksTab'));
console.log('- catalogueTab:', !!document.getElementById('catalogueTab'));

// Check if switchTab function exists
console.log('\nswitchTab function:', typeof switchTab);

// Check for JavaScript errors
if (typeof switchTab === 'undefined') {
    console.error('❌ switchTab function is not defined!');
    
    // Define it manually
    window.switchTab = function(tabName, element) {
        console.log('Using manual switchTab for:', tabName);
        
        // Update buttons
        document.querySelectorAll('.sidebar-tab').forEach(tab => {
            tab.classList.remove('active');
        });
        if (element) element.classList.add('active');
        
        // Update content
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.remove('active');
        });
        
        const targetTab = document.getElementById(tabName + 'Tab');
        if (targetTab) {
            targetTab.classList.add('active');
            console.log('✅ Switched to:', tabName);
        } else {
            console.error('❌ Tab not found:', tabName + 'Tab');
        }
    };
    
    console.log('✅ Manual switchTab function added');
}

// Test clicking a tab
console.log('\n=== TESTING TAB CLICK ===');
if (templatesBtn) {
    console.log('Clicking Templates tab...');
    templatesBtn.click();
} else {
    console.log('❌ Cannot test - Templates button not found');
}

// Check for any JavaScript errors in the page
console.log('\n=== CHECKING FOR ERRORS ===');
// This will show if there are syntax errors preventing scripts from running

// Add event listeners manually as a backup
console.log('\n=== ADDING MANUAL EVENT LISTENERS ===');
document.querySelectorAll('.sidebar-tab').forEach(btn => {
    const tabName = btn.textContent.toLowerCase().trim();
    btn.removeAttribute('onclick'); // Remove broken onclick
    btn.addEventListener('click', function() {
        console.log('Manual click handler for:', tabName);
        switchTab(tabName, this);
    });
});

console.log('\n✅ Manual event listeners added. Try clicking tabs now!');
console.log('=== END DIAGNOSTIC ===');