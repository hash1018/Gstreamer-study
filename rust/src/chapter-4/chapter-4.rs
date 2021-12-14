use gst::prelude::*;
#[allow(unused_imports)]
use std::io;
#[allow(unused_imports)]
use std::io::Write;

#[path = "../common.rs"]
mod common;

struct CustomData {
    /// Our one and only element
    playbin: gst::Element,
    /// Are we in the PLAYING state?
    playing: bool,
    /// Should we terminate execution?
    terminate: bool,
    /// Is seeking enabled for this media?
    seek_enabled: bool,
    /// Have we performed the seek already?
    seek_done: bool,
    /// How long does this media last, in nanoseconds
    duration: Option<gst::ClockTime>,
}

fn tutorial_main() {
    // Initialize GStreamer
    gst::init().unwrap();

    // Creat the playbin element
    let playbin = gst::ElementFactory::make("playbin", Some("playbin"))
        .expect("Failed to create playbin element");

    // Set the URI to play
    let uri =
        "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm";
    playbin.set_property("uri", uri).unwrap();

    // Start playing
    playbin
        .set_state(gst::State::Playing)
        .expect("Unable to set the playbin to the `Playing` state");

    // Listen to the bus
    let bus = playbin.bus().unwrap();
    let mut custom_data = CustomData {
        playbin,
        playing: false,
        terminate: false,
        seek_enabled: false,
        seek_done: false,
        duration: gst::ClockTime::NONE,
    };

    while !custom_data.terminate {
        let msg = bus.timed_pop(100 * gst::ClockTime::MSECOND);

        match msg {
            Some(msg) => {
                handle_message(&mut custom_data, &msg);
            }
            None => {
                if custom_data.playing {

                    /* Query the current position of the stream */
                    let position = custom_data
                        .playbin
                        .query_position::<gst::ClockTime>()
                        .expect("Could not query current position.");

                    // If we didn't know it yet, query the stream duration
                    if custom_data.duration == gst::ClockTime::NONE {
                        custom_data.duration = custom_data.playbin.query_duration();
                    }

                    // Print current position and total duration
                    /*print!(
                        "\rPosition {} / {}",
                        position,
                        custom_data.duration.display()
                    );
                    
                    io::stdout().flush().unwrap();
                    */

                    // /* If seeking is enabled, we have not done it yet, and the time is right, seek */
                    if custom_data.seek_enabled
                        && !custom_data.seek_done
                        && position > 10 * gst::ClockTime::SECOND
                    {
                        println!("\nReached 10s, performing seek...");
                        custom_data
                            .playbin
                            .seek_simple(
                                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                                30 * gst::ClockTime::SECOND,
                            )
                            .expect("Failed to seek.");

                            // gst::SeekFlags::FLUSH: This discards all data currently in the pipeline before doing the seek. 
                            // Might pause a bit while the pipeline is refilled and the new data starts to show up, 
                            // but greatly increases the “responsiveness” of the application. 
                            // If this flag is not provided, “stale” data might be shown for a while until the new position appears at the end of the pipeline.


                            // gst::SeekFlags::KEY_UNIT: With most encoded video streams, 
                            // seeking to arbitrary positions is not possible but only to certain frames called Key Frames. 
                            // When this flag is used, the seek will actually move to the closest key frame and start producing data straight away. 
                            // If this flag is not used, the pipeline will move internally to the closest key frame (it has no other alternative) 
                            // but data will not be shown until it reaches the requested position. This last alternative is more accurate, but might take longer.

                            // gst::SeekFlags::ACCURATE: Some media clips do not provide enough indexing information, 
                            // meaning that seeking to arbitrary positions is time-consuming. 
                            // In these cases, GStreamer usually estimates the position to seek to, and usually works just fine. 
                            // If this precision is not good enough for your case (you see seeks not going to the exact time you asked for), 
                            // then provide this flag. Be warned that it might take longer to calculate the seeking position (very long, on some files).


                        custom_data.seek_done = true;
                    }
                }
            }
        }
    }

    // Shutdown pipeline
    custom_data
        .playbin
        .set_state(gst::State::Null)
        .expect("Unable to set the playbin to the `Null` state");
}

fn handle_message(custom_data: &mut CustomData, msg: &gst::Message) {
    use gst::MessageView;

    match msg.view() {
        MessageView::Error(err) => {
            println!(
                "Error received from element {:?}: {} ({:?})",
                err.src().map(|s| s.path_string()),
                err.error(),
                err.debug()
            );
            custom_data.terminate = true;
        }
        MessageView::Eos(..) => {
            println!("End-Of-Stream reached.");
            custom_data.terminate = true;
        }
        MessageView::DurationChanged(_) => {
            // The duration has changed, mark the current one as invalid
            custom_data.duration = gst::ClockTime::NONE;
        }
        MessageView::StateChanged(state_changed) => {
            if state_changed
                .src()
                .map(|s| s == custom_data.playbin)
                .unwrap_or(false)
            {
                let new_state = state_changed.current();
                let old_state = state_changed.old();

                println!(
                    "Pipeline state changed from {:?} to {:?}",
                    old_state, new_state
                );

                custom_data.playing = new_state == gst::State::Playing;

                // Seeks and time queries generally only get a valid reply when in the PAUSED or PLAYING state, 
                // since all elements have had a chance to receive information and configure themselves. 
                // Here, we use the playing variable to keep track of whether the pipeline is in PLAYING state. 
                // Also, if we have just entered the PLAYING state, we do our first query. We ask the pipeline if seeking is allowed on this stream:

                if custom_data.playing {
                    let mut seeking = gst::query::Seeking::new(gst::Format::Time);
                    if custom_data.playbin.query(&mut seeking) {
                        let (seekable, start, end) = seeking.result();
                        custom_data.seek_enabled = seekable;
                        if seekable {
                            println!("Seeking is ENABLED from {} to {}", start, end)
                        } else {
                            println!("Seeking is DISABLED for this stream.")
                        }
                    } else {
                        eprintln!("Seeking query failed.")
                    }
                }
            }
        }
        _ => (),
    }
}

fn main() {
    // tutorials_common::run is only required to set up the application environment on macOS
    // (but not necessary in normal Cocoa applications where this is set up automatically)
    common::run(tutorial_main);
}