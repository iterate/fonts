# fonts

fontsfontsfonts

## Goal

Create an overview of all fonts in use on the internet. Creatives that work with fonts can use this overview as inspiration for their work. Sort of like [fontsinuse.com](https://fontsinuse.com).

## Personal goal

Create something where I force myself to learn new things.

## The system

- Parse font files to extract metadata
  - [x] wOFF parser
  - [ ] wOF2 parser
  - [ ] OTF parser
  - [ ] TTF parser
- Pipeline from urls to font metadata
  - Fetch and parse html to find the font files
    - Crawlers
      - [x] Request to server
      - [x] Browser
    - Get css content
      - [x] Links to stylesheet
      - [x] Inline css
    - Get fonts defined in font-face attribute in css content
      - [x] Fetch urls defined in css
      - [ ] Parse base64 encoded data defined in css
  - Event-driven architecture to get font metadata from multiple urls ([_Why event driven?_](#why_event_driven))
    - MPMC channels, where a message can only be received by one of all consumers
    - Jobs
      - [x] "Http" job
        - The job that fetches html content by making a reques to the server
      - [x] Browser job
        - The job that fetches html content by visiting the url in a headless browser
      - [x] Verifier job
        - The job that verifies that the html content from the http job has font urls. If it doesnt have any font urls, it sends it to the browser job (because some html content may be dynamically loaded).
      - [x] "Page" job
        - The job that gets html content from either verifier job or browser job, and (currently) outputs font metadata
- Use Common Crawl Index to find urls
  - [ ] Parse urls from one url index file
  - [ ] Figure out how to do reasonably create something where url index files result in urls to visit
    - The file is big (think 230GB)
- Persist font metadata associated with a url
  - [ ] Save data in page job

## Random

<a id="why_event_driven"></a>

### Why event driven?

Because it's something I didn't know much about beforehand, and seemed like a cool way to solve this problem. This also enables me to potentially use NATS in hops, and have multiple pods contribute to this task. Could have done this with just async/await code, if my only goal was to have the differents tasks run asynchronously.
