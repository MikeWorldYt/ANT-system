import { Component, Input } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { CommonModule } from '@angular/common';

// ▲ COMPONENTS ▲
import { FooterComponent } from '../../../layout/footer/footer.component';

// ▲ SERVICES ▲
import { TitleService } from '../../../services/navTitle.service';
import { PageService } from '../../../services/navPage.service';

// ▲ FEATURES ▲
import { ContentPageComponent } from '../../content-page/content-page.component';

@Component({
  selector: 'doc-t02-mod-03-nomenclature',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    FooterComponent,
    ContentPageComponent
  ],
  templateUrl: './page-03.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Pag03_Component {
  // ▲ SERVICES ▲
  constructor(
    private titleService: TitleService,
    private pageService: PageService,
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: string = 'EN';
  currentTitle: string = 'title_02';
  currentPage: string = 'page_03';
  currentArticle: string = '';

  // ████ OnInit ████
  ngOnInit(): void {
    this.titleService.setTitle(this.currentTitle);
    this.pageService.setCurrentPage(this.currentPage);
  }

}