package com.dawmcp.bitwig;

import com.bitwig.extension.controller.ControllerExtension;
import com.bitwig.extension.controller.api.ControllerHost;

/** Bitwig Controller Extension skeleton. Talks JSON to daw-mcp on 127.0.0.1:17302. */
public class DawMcpExtension extends ControllerExtension {
    protected DawMcpExtension(DawMcpExtensionDefinition definition, ControllerHost host) {
        super(definition, host);
    }

    @Override
    public void init() {
        getHost().showPopupNotification("daw-mcp Bitwig bridge");
    }

    @Override
    public void exit() {}

    @Override
    public void flush() {}
}
