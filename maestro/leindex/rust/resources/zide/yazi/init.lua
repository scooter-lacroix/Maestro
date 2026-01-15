-- auto-layout logic (consolidated for stability)
function Tab:layout()
    local area = self._area
    if not area then return end
    
    local r_raw = MANAGER.ratio
    if not r_raw or #r_raw < 3 then r_raw = { 1, 1, 2 } end
    
    local r = {
        parent = r_raw[1],
        current = r_raw[2],
        preview = r_raw[3],
        all = r_raw[1] + r_raw[2] + r_raw[3]
    }

    if area.w > 80 then
        self._chunks = ui.Layout()
            :direction(ui.Layout.HORIZONTAL)
            :constraints({
                ui.Constraint.Ratio(r.parent, r.all),
                ui.Constraint.Ratio(r.current, r.all),
                ui.Constraint.Ratio(r.preview, r.all)
            })
            :split(area)
    elseif area.w > 40 then
        self._chunks = ui.Layout()
            :direction(ui.Layout.HORIZONTAL)
            :constraints({
                ui.Constraint.Ratio(0, r.all),
                ui.Constraint.Ratio(r.current + r.parent, r.all),
                ui.Constraint.Ratio(r.preview + r.parent, r.all)
            })
            :split(area)
    else
        self._chunks = ui.Layout()
            :direction(ui.Layout.HORIZONTAL)
            :constraints({
                ui.Constraint.Ratio(0, r.all),
                ui.Constraint.Ratio(r.all, r.all),
                ui.Constraint.Ratio(0, r.all),
            })
            :split(area)
    end
end

-- Navigation Hints for Zide
function Status:children_render()
	return {
		ui.Span(" [Enter] Open  [Tab] Switch Pane  [q] Quit Zide "):fg("magenta"):bold()
	}
end
