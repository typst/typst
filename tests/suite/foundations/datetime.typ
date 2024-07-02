--- datetime-constructor-empty ---
// Error: 2-12 at least one of date or time must be fully specified
#datetime()

--- datetime-constructor-time-invalid ---
// Error: 2-42 time is invalid
#datetime(hour: 25, minute: 0, second: 0)

--- datetime-constructor-date-invalid ---
// Error: 2-41 date is invalid
#datetime(year: 2000, month: 2, day: 30)

--- datetime-display ---
// Test displaying of dates.
#test(datetime(year: 2023, month: 4, day: 29).display(), "2023-04-29")
#test(datetime(year: 2023, month: 4, day: 29).display("[year]"), "2023")
#test(datetime(year: 2023, month: 4, day: 29).display("[year repr:last_two]"), "23")
#test(datetime(year: 2023, month: 4, day: 29).display("[year] [month repr:long] [day] [week_number] [weekday]"), "2023 April 29 17 Saturday")

#test(datetime(year: 2024, month: 7, day: 2).display(locale: "de", date: "full"), "Dienstag, 2. Juli 2024")
#test(datetime(year: 2024, month: 7, day: 2).display(locale: "de", date: "long"), "2. Juli 2024")
#test(datetime(year: 2024, month: 7, day: 2).display(locale: "de", date: "medium"), "02.07.2024")
#test(datetime(year: 2024, month: 7, day: 2).display(locale: "de", date: "short"), "02.07.24")
#test(datetime(year: 2024, month: 7, day: 2).display(locale: "th", date: "long"), "2 กรกฎาคม 2567")
#test(datetime(year: 2024, month: 7, day: 2).display(locale: "bg", date: "medium"), "2.07.2024\u{202f}г.")
#test(datetime(year: 2024, month: 7, day: 2).display(locale: "bg"), "2.07.2024\u{202f}г.")
#test(datetime(year: 2024, month: 7, day: 2).display(locale: "bn_IN", date: "short"), "২/৭/২৪")

// Test displaying of times
#test(datetime(hour: 14, minute: 26, second: 50).display(), "14:26:50")
#test(datetime(hour: 14, minute: 26, second: 50).display("[hour]"), "14")
#test(datetime(hour: 14, minute: 26, second: 50).display("[hour repr:12 padding:none]"), "2")
#test(datetime(hour: 14, minute: 26, second: 50).display("[hour], [minute], [second]"), "14, 26, 50")
#test(datetime(hour: 14, minute: 26, second: 50).display(locale: "en-US", time: "medium"), "2:26:50\u{202f}PM")
#test(datetime(hour: 14, minute: 26, second: 50).display(locale: "ar", time: "short"), "٢:٢٦ م")

// Test displaying of datetimes
#test(datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50).display(), "2023-04-29 14:26:50")
#test(datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50).display(locale: "es"), "29 abr 2023, 14:26:50")
#test(datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50).display(locale: "vi", date: "full"), "14:26:50 Thứ Bảy, 29 tháng 4, 2023")

// Test getting the year/month/day etc. of a datetime
#let d = datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50)
#test(d.year(), 2023)
#test(d.month(), 4)
#test(d.weekday(), 6)
#test(d.day(), 29)
#test(d.hour(), 14)
#test(d.minute(), 26)
#test(d.second(), 50)

#let e = datetime(year: 2023, month: 4, day: 29)
#test(e.hour(), none)
#test(e.minute(), none)
#test(e.second(), none)

// Test today
#test(datetime.today().display(), "1970-01-01")
#test(datetime.today(offset: auto).display(), "1970-01-01")
#test(datetime.today(offset: 2).display(), "1970-01-01")

--- datetime-ordinal ---
// Test date methods.
#test(datetime(day: 1, month: 1, year: 2000).ordinal(), 1);
#test(datetime(day: 1, month: 3, year: 2000).ordinal(), 31 + 29 + 1);
#test(datetime(day: 31, month: 12, year: 2000).ordinal(), 366);
#test(datetime(day: 1, month: 3, year: 2001).ordinal(), 31 + 28 + 1);
#test(datetime(day: 31, month: 12, year: 2001).ordinal(), 365);

--- datetime-display-missing-closing-bracket ---
// Error: 27-34 missing closing bracket for bracket at index 0
#datetime.today().display("[year")

--- datetime-display-invalid-component ---
// Error: 27-38 invalid component name 'nothing' at index 1
#datetime.today().display("[nothing]")

--- datetime-display-invalid-modifier ---
// Error: 27-50 invalid modifier 'wrong' at index 6
#datetime.today().display("[year wrong:last_two]")

--- datetime-display-expected-component ---
// Error: 27-33 expected component name at index 2
#datetime.today().display("  []")

--- datetime-display-insufficient-information ---
// Error: 2-36 failed to format datetime (insufficient information)
#datetime.today().display("[hour]")

--- datetime-display-bad-date-length ---
// Error: 47-52 expected "full", "long", "medium", or "short"
#datetime.today().display(locale: "en", date: "foo")

--- datetime-display-bad-time-length ---
// Error: 47-53 expected "medium" or "short"
#datetime.today().display(locale: "en", time: "long")
