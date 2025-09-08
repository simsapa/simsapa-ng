import * as h from "./helpers";
import { findManager } from "./find";

document.SSP = {
    show_transient_message: h.show_transient_message,
    find: findManager,
};
