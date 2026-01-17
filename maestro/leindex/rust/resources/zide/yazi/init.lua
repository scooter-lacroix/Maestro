-- Auto-layout logic for Yazi when used as a Zide picker pane.
-- Keep this defensive: Yazi's Lua runtime changed over time, and a bad layout
-- hook can result in a blank UI even though keybinds still work.
local original_tab_layout = Tab.layout

local function get_ratio()
	-- Yazi v26+ provides a computed ratio table at rt.mgr.ratio.
	local r = (rt and rt.mgr and rt.mgr.ratio) or nil
	if type(r) == "table" and r.parent and r.current and r.preview and r.all then
		return r
	end

	-- Back-compat for older configs that used MANAGER.ratio as an array.
	local raw = (MANAGER and MANAGER.ratio) or nil
	if type(raw) == "table" and raw[1] and raw[2] and raw[3] then
		return {
			parent = raw[1],
			current = raw[2],
			preview = raw[3],
			all = raw[1] + raw[2] + raw[3],
		}
	end

	return { parent = 1, current = 1, preview = 2, all = 4 }
end

function Tab:layout()
	local area = self._area
	if not area then
		if original_tab_layout then
			return original_tab_layout(self)
		end
		return
	end

	local r = get_ratio()

	local ok, chunks = pcall(function()
		if area.w > 80 then
			return ui.Layout()
				:direction(ui.Layout.HORIZONTAL)
				:constraints({
					ui.Constraint.Ratio(r.parent, r.all),
					ui.Constraint.Ratio(r.current, r.all),
					ui.Constraint.Ratio(r.preview, r.all),
				})
				:split(area)
		elseif area.w > 40 then
			return ui.Layout()
				:direction(ui.Layout.HORIZONTAL)
				:constraints({
					ui.Constraint.Ratio(0, r.all),
					ui.Constraint.Ratio(r.current + r.parent, r.all),
					ui.Constraint.Ratio(r.preview + r.parent, r.all),
				})
				:split(area)
		else
			return ui.Layout()
				:direction(ui.Layout.HORIZONTAL)
				:constraints({
					ui.Constraint.Ratio(0, r.all),
					ui.Constraint.Ratio(r.all, r.all),
					ui.Constraint.Ratio(0, r.all),
				})
				:split(area)
		end
	end)

	if not ok or not chunks then
		if original_tab_layout then
			return original_tab_layout(self)
		end
		return
	end

	self._chunks = chunks
end
