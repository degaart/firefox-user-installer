#pragma once

#include <wx/wx.h>

namespace fui {

	class App : public wxApp {
		public:
			bool OnInit() override;
	};

}

wxDECLARE_APP(fui::App);

