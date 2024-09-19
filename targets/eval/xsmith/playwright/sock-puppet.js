/**
 * @name Sock Puppet
 * 
 * This script will launch a browser and give it instructions to run the load-playwright script,
 * which all cumulates in the browser running some WebAssembly code.
 * This is the script you want to run from the command line or fuzzing campaign.
 *
 * @desc Goes to webpage that will execute a given wasm program and returns the result.
 *
 * To debug: 
 *  - !! First make sure the server is running: !!
 *      `python3 -m http.server --directory ./playwright/ 8080 &`
 *  - Run with node: `node --inspect-brk sock-puppet.js -b <browser_binary> -p http://localhost:8080/example.html -f <wasm-file>`
 *  - Open up the browser and navigate to the debug window to connect the debugger.
 *    - On chrome, you might have to hit the little green node symbol in the top left of the debug
 *      tools 
 */
const playwright = require('playwright');
const path = require('path');
const os = require('os');
const fs = require('fs');
const util = require('util');


const HEADLESS = true; //false is helpful for debugging
const AUTOCLOSE = true; //If true, will auto-close the browser once a response comes in on the console
var playwright_timeout = 30000; //30 seconds
if (!AUTOCLOSE) {
  playwright_timeout = 0;
}
// Parse command line arguments
let argv = require('yargs/yargs')(process.argv.slice(2))
    .usage('Usage: $0 [options]')
    .example('$0 sock-puppet.js -b /usr/bin/firefox -p http://localhost:8080/wasm-webpage.html -f /local/work/test_wasm.wasm', 
      'test Firefox with the given webage and wasm program')
    .alias('b', 'browser')
    .nargs('b', 1)
    .describe('b', 'Browser to use. Valid options are \'chromium\' or \'firefox\'')
    .alias('p', 'page')
    .nargs('p', 1)
    .describe('p', 'URL of webpage (remember to start the webserver)')
    .alias('f', 'file')
    .nargs('f', 1)
    .describe('f', 'Path to Wasm file to run, relative from where the http webserver was started')
    .nargs('local')
    .describe('local', 'Use for testing locally. Will not attempt to make profile directories or anything past the script directory')
    .demandOption(['b', 'p', 'f'])
    .help('h')
    .alias('h', 'help') 
    .argv;

if (argv.browser != 'firefox' && argv.browser != 'chromium') {
  console.log("Browser option must be either 'firefox' or 'chromium'.")
  process.exit(1)
}

let url = argv.page;
let program_path = argv.file;
let local = argv.local;

// Firefox doesn't like having too many of the same profile open at the same time. Instead, we can
// make a dummy profile for every launch
// https://github.com/puppeteer/puppeteer/issues/3737

var temporaryProfileDir;
if (argv.browser == "firefox" && !local) {
  fs.mkdirSync('/local/work/firefox_profiles/', {recursive: true});
  temporaryProfileDir = fs.mkdtempSync(path.join('/local/work/firefox_profiles/','playwright_temp_profile-'));
  fs.copyFileSync(
    '/local/work/webassembly-sandbox/playwright/prefs.js',
    path.join(temporaryProfileDir, 'prefs.js'));
}

function cleanup(profile_dir) {
  if (!local) {
    fs.rmdirSync(profile_dir, {recursive: true});
  }
}

(async () =>{
  try {
    var browser_args = []
    if (argv.browser == "firefox") {
      // browser_args = ["-wait-for-browser"];
      //if (!local) {
        //browser_args.push("-profile", temporaryProfileDir);
      //}
    } else { // chromium
      browser_args = ["--no-sandbox"];
    }
    var browser = null;
    var context = null;
    if (argv.browser == "firefox" && local) {
      browser = await playwright.firefox.launch({
        //slowMo: 250, // slow down by 250ms
        headless: HEADLESS,
        timeout: playwright_timeout,
        args: browser_args});
      context = await browser.newContext();
    } else if (argv.browser == "firefox" && !local) {
       context = await playwright.firefox.launchPersistentContext(temporaryProfileDir, {
        //slowMo: 250, // slow down by 250ms
        headless: HEADLESS,
        timeout: playwright_timeout,
        args: browser_args});
    } else {
      browser = await playwright.chromium.launch({
        //slowMo: 250, // slow down by 250ms
        headless: HEADLESS,
        timeout: playwright_timeout,
        args: browser_args});
      context = await browser.newContext();
    }
    const page = await context.newPage();

    page.on('console', message => {
      if (message.type() === 'log') { 
        console.log(message.text());
        if (AUTOCLOSE) {
          if (argv.browser == "firefox") {
            cleanup(temporaryProfileDir);
          }
          if (argv.browser == "firefox" && !local) {
            context.close();
          } else {
            browser.close();
          }

          process.exit();
        }
      } else if (message.type() === 'verbose') { // Old versions of Firefox are special
        console.log(message.text()[0]);
        if (AUTOCLOSE) {
          if (argv.browser == "firefox") {
            cleanup(temporaryProfileDir);
          }
          if (argv.browser == "firefox" && !local) {
            context.close();
          } else {
            browser.close();
          }

          process.exit();
        }
      }
    });

    await page.goto(url);
    await page.locator("#wasm-program").fill(String(program_path));
    await page.locator("#run-button").click();

    await page.waitForTimeout(playwright_timeout);
    if (AUTOCLOSE) {
      if (argv.browser == "firefox") {
        cleanup(temporaryProfileDir);
      }
      if (argv.browser == "firefox" && !local) {
        await context.close();
      } else {
        await browser.close();
      }
      await process.exit();
    } else { // If not autoclose, clean up anyway, but let the browser and this process keep running
      if (argv.browser == "firefox") {
        cleanup(temporaryProfileDir);
      }
    }
  } catch (e) { // clean up profile dir if an exception happens too
    console.log(e)
    if(argv.browser == "firefox") {
      cleanup(temporaryProfileDir);
    }
  }
})();

