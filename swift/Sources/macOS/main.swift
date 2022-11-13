import Cocoa
import Foundation
import ORSSerial


let (app, delegate) = (NSApplication.shared, App.Delegate())
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
