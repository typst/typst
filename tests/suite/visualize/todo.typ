// Test the `todo` element.

--- todo ---
// Default todo.

// Warning: 2-8 TODO
#todo()

--- todo-message ---
// Test message.

// Warning: 2-22 TODO: hey
#todo(message: "hey")

--- todo-no-warn ---
// Test no warning.

#todo(warn: false) \
#todo(warn: false, message: "hey")

--- todo-set-no-warn ---
// Test with no warning using set.

#set todo(warn: false)
#todo() \
#todo(message: "hey")

--- todo-set-warn ---
// Test with and w/o warning using set.

#set todo(warn: false)
#todo() \
#todo(message: "hey") \

#set todo(warn: true)
// Warning: 2-8 TODO
#todo() \
// Warning: 2-22 TODO: hey
#todo(message: "hey")

--- todo-show-rule ---
// Test show rule.

#show todo: set text(fill: blue)

// Warning: 2-8 TODO
#todo() \
// Warning: 2-22 TODO: hey
#todo(message: "hey")
